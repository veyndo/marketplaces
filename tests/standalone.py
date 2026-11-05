"""Run the released gateway against an independent in-memory host, without Veyndo or PostgreSQL."""
import copy, json, os, socket, subprocess, threading, time, urllib.request, urllib.error
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
TOKEN = 'fixture-gateway-identity-0000000000'
CALLBACK = 'fixture-host-identity-00000000000'
listing = {'id':'offer','revision':1,'connection_id':'seller','product_id':'lamp','sku':'LAMP','images':['https://example.test/lamp.jpg'],'state':'draft','title':{},'mapping':{}}
connection = {'id':'seller','revision':1,'active':True,'account':'seller','provider':'amazon','channel_id':'de','currency':'EUR','locale':'it-IT','policy':{'productType':'HOME'}}
product = {'id':'lamp','revision':9,'mainLocale':'fr-FR','translations':{'fr-FR':{'name':'Lampe','description':'Description française'}},'commerce':{'stock':8,'media':[]}}
rows = {'own':{'listings':[listing],'connections':[connection]},'other':{'listings':[],'connections':[]}}
lock = threading.Lock()
calls = []
jobs = {}
class Host(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_POST(self):
        tenant = self.headers.get('x-tenant')
        assert self.headers.get('Authorization') == 'Bearer ' + CALLBACK
        assert self.headers.get('x-veyndo-host-version') == '1'
        payload = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        calls.append((tenant, self.path, copy.deepcopy(payload)))
        status = 200
        with lock:
            if self.path.endswith('/record_action'):
                action, args = payload['action'], payload['input']
                verb, entity = action.split('_',1)
                matches = [r for r in rows[tenant][entity] if 'id' not in args or r['id'] == args['id']]
                if verb == 'list': result = {'elements':matches,'hasMore':False}
                elif verb == 'save':
                    row = next((r for r in matches if r['id'] == args['id']),None)
                    if row is None or row['revision'] != args['revision']:
                        status, result = 409, {'error':'Revision conflict'}
                    else:
                        row.update(args['fields']);row['revision'] += 1;result = copy.deepcopy(row)
                else: raise AssertionError(action)
            elif self.path.endswith('/job_enqueue'):
                key = (tenant, payload['requestKey'])
                if key in jobs and jobs[key]['input'] != payload['input']:
                    status, result = 409, {'error':'Request key conflict'}
                else:
                    reused = key in jobs
                    jobs.setdefault(key, {'id':'job-'+str(len(jobs)), 'input':payload['input']})
                    result = {'accepted':True,'jobId':jobs[key]['id'],'reused':reused}
            elif self.path.endswith('/channel'):
                result = {'data':{'locales':['fr-FR','de-DE','en-GB','es-ES','it-IT']},'currencies':{'enabled':['EUR']}}
            elif self.path.endswith('/channel_product'):
                result = {'product':product,'money':{'minor':1299,'currency':{'code':'EUR','scale':2}}}
            else: raise AssertionError(self.path)
            body = json.dumps(result).encode()
        self.send_response(status);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(body)
def call(base,path,data=None,token=TOKEN,tenant='own',status=200):
    request = urllib.request.Request(base+path, data=None if data is None else json.dumps(data).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+token,'x-tenant':tenant})
    try: response = urllib.request.urlopen(request,timeout=10)
    except urllib.error.HTTPError as e: response = e
    result = json.load(response)
    assert response.status == status,(response.status,result)
    return result
host = ThreadingHTTPServer(('127.0.0.1',0),Host)
threading.Thread(target=host.serve_forever,daemon=True).start()
with socket.socket() as probe: probe.bind(('127.0.0.1',0));port = probe.getsockname()[1]
url = f'http://127.0.0.1:{port}'
tenants = {t:{'coreUrl':f'http://127.0.0.1:{host.server_port}','callbackToken':CALLBACK,'hostPath':'/commerce/v1/','accounts':{'seller':{'provider':'amazon'}}} for t in rows}
env = {**os.environ,'APP_BIND_ADDR':f'127.0.0.1:{port}','APP_TOKEN':TOKEN,'APP_MARKETPLACE_TENANTS':json.dumps(tenants)}
process = subprocess.Popen([os.environ.get('MARKETPLACE_BINARY',str(ROOT/'target/debug/marketplace-gateway'))],env=env)
try:
    for _ in range(100):
        assert process.poll() is None,'service exited'
        try: call(url,'/health');break
        except OSError: time.sleep(.05)
    else: raise AssertionError('service did not start')
    call(url,'/actions/prepare',{'listingId':'offer'},token='wrong',status=401)
    call(url,'/mcp',{'jsonrpc':'2.0','id':1,'method':'tools/list'},tenant='foreign',status=403)
    assert not calls,'denied calls reached host'
    rpc = lambda method,params={}: call(url,'/mcp',{'jsonrpc':'2.0','id':7,'method':method,'params':params})
    assert rpc('initialize')['result']['serverInfo']['name'] == 'veyndo-marketplaces'
    catalog = rpc('tools/list')['result']['tools']
    assert {t['name'] for t in catalog} == {a['name'] for a in json.loads((ROOT/'manifest.json').read_text())['actions'] if a['mcp']}
    assert 'requestKey' in next(t for t in catalog if t['name']=='sync')['inputSchema']['required']
    assert next(t for t in catalog if t['name']=='prepare')['inputSchema']['required'] == ['listingId']
    assert rpc('unknown')['error']['code'] == -32601
    assert rpc('tools/call',{'name':'nope','arguments':{}})['error']['code'] == -32602
    first = call(url,'/actions/prepare',{'listingId':'offer'})
    second = rpc('tools/call',{'name':'prepare','arguments':{'listingId':'offer'}})['result']
    assert first == second['structuredContent'] and not second['isError']
    assert first['mapped']['title'] == 'Lampe' and first['mapped']['locale']=='it-IT'
    assert first['mapped']['money']['minor']==1299 and first['mapped']['sourceRevision']==9
    assert listing['revision']==3 and listing['state']=='ready'
    assert rpc('tools/call',{'name':'prepare','arguments':{}})['result']['isError']
    call(url,'/actions/prepare',{'listingId':'offer'},tenant='other',status=404)
    assert rows['other']['listings']==[]
    assert rpc('tools/call',{'name':'import_order','arguments':{'approve':False}})['result']['isError']
    assert call(url,'/actions/list_connections',{})['elements'][0]['id']=='seller'
    args = {'connectionId':'seller','kind':'orders','requestKey':'one-request'}
    accepted = call(url,'/actions/sync',args)
    replay = rpc('tools/call',{'name':'sync','arguments':args})['result']['structuredContent']
    assert accepted['jobId']==replay['jobId'] and not accepted['reused'] and replay['reused']
    call(url,'/actions/sync',{**args,'kind':'listing'},status=409)
    call(url,'/actions/sync',{'connectionId':'seller','kind':'orders'},status=400)
    call(url,'/actions/save_listings',{'id':'offer','revision':0,'fields':{}},status=409)
    assert all(path.startswith('/commerce/v1/') for _,path,_ in calls)
    if os.environ.get('MARKETPLACE_SDK_EXAMPLE'):
        subprocess.run(['node','examples/prepare.mjs'],env={**env,'MARKETPLACE_URL':url,'MARKETPLACE_TENANT':'own','MARKETPLACE_TOKEN':TOKEN,'MARKETPLACE_LISTING':'offer'},check=True,cwd=ROOT)
        subprocess.run(['node','examples/host.mjs'],env={**env,'MARKETPLACE_HOST_URL':f'http://127.0.0.1:{host.server_port}','MARKETPLACE_TENANT':'own','MARKETPLACE_HOST_TOKEN':CALLBACK},check=True,cwd=ROOT)
    print('PASS standalone HTTP/MCP, shared effects, authentication, tenant isolation, exact money, fifth language, SDK and rejected imports')
finally:
    process.terminate();process.wait(timeout=10);host.shutdown();host.server_close()
