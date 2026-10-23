"""Native external-order language, batching, snapshot and stock regressions through the installed app callback."""
import concurrent.futures
import json
import subprocess
from testing.database import psql


def verify(call, base, owner, foreign, credential, snapshot, env, log):
    tenant = owner['x-tenant']

    def quoted(value):
        return "'" + value.replace("'", "''") + "'"

    def sql(statement):
        return subprocess.check_output(psql(env['TEST_DB_CONTAINER'], 'commerce', env['TEST_DATABASE'],
                                           '-qAt', '-v', 'ON_ERROR_STOP=1'), input=statement, text=True).strip()

    settings = call(base, '/api/merchant/commerce', h=owner)
    settings['data']['locales'].append('it-IT')
    call(base, '/api/merchant/commerce', settings, owner, method='PUT')
    channel = {'name': {'en-GB': 'Import regression'}, 'kind': 'marketplace', 'active': True,
               'visibility': 'public', 'locales': ['it-IT'], 'productIds': []}
    call(base, '/api/automation/channels/import_regression', {'revision': 0, 'data': channel}, owner, method='PUT')
    # Synthetic catalogue only. Both shops deliberately share product IDs; all real
    # imports below still pass installed-app authentication and the native HTTP path.
    sql("INSERT INTO languages(id,locale) VALUES('import-it','it') ON CONFLICT(locale) DO NOTHING;")
    expected = {}
    for scope in [tenant, foreign['x-tenant']]:
        for index in range(1, 25):
            parent, child = f'import-parent-{index}', f'import-child-{index}'
            for product, parent_id in [(parent, None), (child, parent)]:
                sql(f"INSERT INTO products(tenant,id,name,category,description,price,tax_rate,stock,parent_id) "
                    f"VALUES({quoted(scope)},{quoted(product)},{quoted('Raw '+product)},'fixture','',10,0,100,"
                    f"{quoted(parent_id) if parent_id else 'NULL'});")
            case = index % 8
            names = {
                0: [(child, 'it-IT', 'Figlio'), (parent, 'it-IT', 'Genitore')],
                1: [(child, 'en-GB', 'Child English'), (parent, 'it-IT', 'Genitore')],
                2: [(child, 'it', 'Nome breve'), (parent, 'en-GB', 'Parent English')],
                3: [(child, 'it-IT', '   '), (child, 'en-GB', 'Child English')],
                4: [],
                5: [(child, 'it-IT', None), (parent, 'it-IT', 'Genitore')],
                6: [(child, 'it-IT', '\t'), (parent, 'it-IT', 'Genitore')],
                7: [(parent, 'en-GB', 'Parent English')],
            }[case]
            if scope != tenant:
                names = [(child, 'it-IT', 'Foreign private name')]
            for product, locale, name in names:
                sql(f"INSERT INTO product_translations(tenant,product_id,language_id,name,description) "
                    f"SELECT {quoted(scope)},{quoted(product)},id,{quoted(name) if name is not None else 'NULL'},'' "
                    f"FROM languages WHERE locale={quoted(locale)};")
            expected[child] = {0: 'Figlio', 1: 'Genitore', 2: 'Nome breve', 3: 'Child English',
                               4: 'Raw '+child, 5: 'Genitore', 6: 'Raw '+child, 7: 'Parent English'}[case]

    path = '/api/apps/marketplaces/core/order_import'

    def payload(count, key):
        return {**snapshot, 'salesChannelId': 'import_regression', 'externalId': key, 'locale': 'it-IT',
                'lines': [{'productId': f'import-child-{i}', 'quantity': 1,
                           'amount': {'grossMinor': 1000, 'taxMinor': 0, 'taxRateBasisPoints': 0}}
                          for i in range(1, count+1)], 'totalMinor': count*1000}

    def import_order(body, status=200):
        offset = log.stat().st_size
        result = call(base, path, body, credential, status)
        # SQLx's existing statement tracing counts actual completed database reads.
        queries = sum('sqlx::query' in line and 'product_translations' in line and 'JOIN languages' in line
                      for line in log.read_bytes()[offset:].decode().splitlines())
        return result, queries

    counts = []
    orders = []
    for count in [1, 20, 24]:
        body = payload(count, f'localized-{count}')
        result, queries = import_order(body)
        order = result['order']
        assert not result['replayed']
        assert {line['id']: line['label'] for line in order['cart']['lineItems']} == {
            f'import-child-{i}': expected[f'import-child-{i}'] for i in range(1, count+1)}
        assert order['cart']['price']['totalMinor'] == str(count*1000)
        assert order['payment']['externallyPaid'] and not order['payment']['realMoneyCharged']
        assert sql(f"SELECT count(*) FROM outbox WHERE tenant={quoted(tenant)} AND kind='order.placed' "
                   f"AND data->>'orderId'={quoted(order['id'])};") == '1'
        counts.append(queries)
        orders.append((body, order))
    print('MEASURE external order translation SELECTs for 1/20/24 lines:', counts, flush=True)
    # Frozen names and prices survive later catalogue edits and idempotent replay.
    sql(f"UPDATE product_translations SET name='Changed later' WHERE tenant={quoted(tenant)} "
        "AND product_id LIKE 'import-%';")
    for body, order in orders:
        replay, queries = import_order(body)
        assert replay['replayed'] and replay['order']['id'] == order['id'] and queries == 0, (replay, queries)
        assert replay['order']['cart'] == order['cart'] and replay['order']['payment'] == order['payment']
    stocks = json.loads(sql(f"SELECT json_object_agg(id,stock ORDER BY id) FROM products WHERE tenant={quoted(tenant)} "
                            "AND id LIKE 'import-child-%';"))
    assert stocks == {f'import-child-{i}': 100 - (i == 1) - (i <= 20) - 1 for i in range(1, 25)}
    import_order(payload(25, 'over-limit'), 400)
    # Failing on the final line leaves all earlier stock and order effects intact.
    sql(f"UPDATE products SET stock=0 WHERE tenant={quoted(tenant)} AND id='import-child-24';")
    before = sql(f"SELECT json_object_agg(id,stock ORDER BY id) FROM products WHERE tenant={quoted(tenant)};")
    import_order(payload(24, 'insufficient-last-line'), 409)
    assert sql(f"SELECT json_object_agg(id,stock ORDER BY id) FROM products WHERE tenant={quoted(tenant)};") == before
    assert sql(f"SELECT count(*) FROM orders WHERE tenant={quoted(tenant)} "
               "AND data->'external'->>'orderId' IN ('over-limit','insufficient-last-line');") == '0'
    # Two independently identified orders compete for the final unit under real locks.
    sql(f"UPDATE products SET stock=1 WHERE tenant={quoted(tenant)} AND id='import-child-1';")
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(lambda key: call(base, path, payload(1, key), credential, (200, 409)),
                                ['last-unit-a', 'last-unit-b']))
    assert sum('order' in result for result in results) == 1
    assert sql(f"SELECT stock FROM products WHERE tenant={quoted(tenant)} AND id='import-child-1';") == '0'
    print('PASS fifth-language/short-locale and variant precedence, NULL/blank names, foreign-shop names, '
          'frozen replays, exact snapshots, single outbox effects, bounded input and atomic concurrent stock')
    assert counts == [1, 1, 1], ('Translation reads must stay constant across admitted line counts', counts)
