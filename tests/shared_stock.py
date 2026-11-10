"""Native web checkout, POS and a Marketplace callback share one stock owner."""

from concurrent.futures import ThreadPoolExecutor
from threading import Barrier


def verify(call, base, owner, callback, snapshot, suffix):
    product_id = snapshot['lines'][0]['productId']
    product_path = '/api/merchant/products/' + product_id
    product = call(base, product_path, h=owner)
    product['commerce']['stock'] = 1
    call(base, product_path, product, owner, method='PUT')

    register = {
        'name': {'en-GB': 'Shared stock register'},
        'kind': 'pos',
        'active': True,
        'locales': ['en-GB', 'de-DE'],
        'productIds': [],
        'pos': {'country': 'DE', 'columns': 4, 'compact': False, 'receiptMessage': {}},
    }
    call(base, '/api/automation/channels/shared_register',
         {'revision': 0, 'data': register}, owner, method='PUT')
    pos = '/api/pos/shared_register/'
    cart = call(base, pos + 'create', {}, owner)['cart']
    cart = call(base, pos + 'replace', {
        'cartToken': cart['token'], 'revision': cart['revision'],
        'items': [{'id': product_id, 'quantity': 1}],
    }, owner)
    amount = cart['price']['totalMinor']
    cash = {
        'cartToken': cart['token'], 'revision': cart['revision'],
        'requestKey': 'last-pos-' + suffix, 'totalMinor': amount, 'tenderMinor': amount,
    }

    public = {'x-tenant': owner['x-tenant']}
    web_key = 'last-web-' + suffix
    cart = call(base, '/store-api/checkout/cart', {'session': web_key}, public)
    web = {**public, 'sw-context-token': cart['token'], 'Idempotency-Key': web_key}
    call(base, '/store-api/checkout/cart/line-item',
         {'items': [{'referencedId': product_id, 'quantity': 1}]}, web)
    buyer = {'email': web_key + '@example.test', 'password': 'Synthetic-Shared-Stock-2026!'}
    call(base, '/store-api/account/register', {'name': 'Shared stock buyer', **buyer}, public)
    login = call(base, '/store-api/account/login', buyer, web)
    web['sw-context-token'] = login['token']

    external = {
        **snapshot, 'externalId': 'last-market-' + suffix, 'totalMinor': 1000,
        'lines': [{'productId': product_id, 'quantity': 1,
                   'amount': {'grossMinor': 1000, 'taxMinor': 160, 'taxRateBasisPoints': 1900}}],
    }
    attempts = {
        'register': lambda: call(base, pos + 'complete', cash, owner, (200, 409)),
        'store': lambda: call(base, '/store-api/checkout/order', {}, web, (200, 409)),
        'marketplace': lambda: call(base, '/api/apps/marketplaces/core/order_import',
                                    external, callback, (200, 409)),
    }
    order_path = '/api/merchant/orders?limit=100'
    before = len(call(base, order_path, h=owner)['elements'])
    barrier = Barrier(len(attempts))

    def complete(channel):
        barrier.wait(timeout=5)
        return channel, attempts[channel]()

    def order_id(response):
        return (response.get('order') or response).get('id')

    with ThreadPoolExecutor(max_workers=len(attempts)) as pool:
        results = dict(pool.map(complete, attempts))
    winners = [(channel, order_id(result)) for channel, result in results.items() if order_id(result)]
    assert len(winners) == 1, results
    winner, original = winners[0]
    for channel, result in results.items():
        if channel != winner:
            error = result['errors'][0]
            assert error['status'] == 409 and 'stock' in error['detail'].lower(), (channel, error)
    assert order_id(attempts[winner]()) == original
    assert len(call(base, order_path, h=owner)['elements']) == before + 1
    assert call(base, product_path, h=owner)['commerce']['stock'] == 0
    print('PASS register, web checkout and Marketplace share one last unit and one replayable order')
