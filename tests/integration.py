"""Installed Marketplace app -> independent Rust service -> fixture seller APIs -> native PostgreSQL order effects. No external credentials or requests."""
import json,os,socket,subprocess,sys,time,uuid,urllib.request,urllib.error,threading,concurrent.futures,hmac,hashlib
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];sys.path.insert(0,str(ROOT/'scripts'))
from testing.app_approval import consent,pin
from testing.runtime import serve,stop
sys.path.insert(0,str(Path(__file__).resolve().parent))
from order_localization import verify as verify_order_localization
suffix=uuid.uuid4().hex[:8];env=dict(os.environ);package=json.loads((ROOT/'extensions/apps/marketplaces/manifest.json').read_text());gateway='synthetic-marketplace-'+suffix+'x'*30

def port():
 with socket.socket()as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
core_url=f'http://127.0.0.1:{port()}';adapter_url=f'http://127.0.0.1:{port()}';seller_url=f'http://127.0.0.1:{port()}';requests=[];kaufland_units={}
class Seller(BaseHTTPRequestHandler):
 def log_message(self,*a):pass
 def do_GET(self):self.reply()
 def do_POST(self):self.reply()
 def do_PUT(self):self.reply()
 def do_PATCH(self):self.reply()
 def reply(self):
  body=self.rfile.read(int(self.headers.get('Content-Length','0'))).decode();requests.append((self.command,self.path,body,dict(self.headers)))
  if self.path.startswith('/v2/'):
   signature=hmac.new(b'fixture-secret',f'{self.command}\n{seller_url}{self.path}\n{body}\n{self.headers["Shop-Timestamp"]}'.encode(),hashlib.sha256).hexdigest();assert signature==self.headers['Shop-Signature']
  v={};code=200
  if 'token' in self.path:v={'access_token':'fixture-access','expires_in':3600}
  elif '/listings/' in self.path:v={'status':'ACCEPTED','submissionId':'fixture-listing'}
  elif '/orders/2026' in self.path or '/fulfillment/v1/order' in self.path:v={'orders':[{'orderId':'external-1','lineItems':[{'sku':'LAMP','quantity':2}]}]}
  elif self.path.startswith('/v2/orders/external-1'):v={'data':{'id_order':'external-1','order_units':[{'id_order_unit':1,'id_offer':'LAMP'}]}}
  elif self.path.startswith('/v2/orders'):v={'data':[{'id_order':'external-1','order_units':[]}], 'pagination':{'offset':0,'limit':10,'total':1}}
  elif self.command=='GET' and '/offer?' in self.path:code=404;v={'errors':[{'errorId':25713}]}
  elif self.command=='POST' and self.path.endswith('/offer'):v={'offerId':'fixture-offer'}
  elif self.path.endswith('/publish'):v={'listingId':'fixture-live'}
  elif self.command=='GET' and '/units?' in self.path:
   from urllib.parse import urlsplit,parse_qs
   sku=parse_qs(urlsplit(self.path).query)['id_offer'][0];v={'data':([kaufland_units[sku]] if sku in kaufland_units else [])}
  elif self.path.startswith('/v2/units'):
   unit=json.loads(body);assert isinstance(unit['id_shipping_group'],int) and unit['vat_indicator']=='standard_rate' and 'storefront' not in unit
   if self.command=='POST':kaufland_units[unit['id_offer']]={'id_unit':11}
   if self.command=='PATCH':assert 'ean' not in unit and 'id_offer' not in unit and 'condition' not in unit
   v={'id_unit':11}
  self.send_response(code);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(json.dumps(v).encode())
server=ThreadingHTTPServer(('127.0.0.1',int(seller_url.rsplit(':',1)[1])),Seller);threading.Thread(target=server.serve_forever,daemon=True).start()
def call(base,path,data=None,h=None,status=200,method=None):
 if path=='/api/apps':data=consent(data)
 req=urllib.request.Request(base+path,data=None if data is None else json.dumps(data).encode(),headers={'Content-Type':'application/json',**(h or {})},method=method)
 try:r=urllib.request.urlopen(req,timeout=30)
 except urllib.error.HTTPError as e:r=e
 v=r.read() if path.endswith('/pdf') else json.load(r);assert r.status in (status if isinstance(status,tuple)else(status,)),(path,r.status,status,v);return v
core_env={**env,'RUST_LOG':'sqlx::query=debug','BIND_ADDR':core_url.removeprefix('http://'),'BASE_URL':core_url,'APP_SERVICES':json.dumps(pin({'marketplaces':{'url':adapter_url,'token':gateway}},'marketplaces',package)),'ALLOW_BOOTSTRAP_AUTH':'false'}
(ROOT/'artifacts').mkdir(exist_ok=True);log=(ROOT/'artifacts/marketplaces.log').open('w');core=adapter=None
try:
 core=serve(core_env,core_url,log);tenants={};owners={}
 for label in ['market','market-foreign']:
  r=call(core_url,'/api/auth/register',{'workspaceId':label+'-'+suffix,'name':'Marketplace fixture','email':label+suffix+'@example.test','password':'Synthetic-Marketplace-fixture-2026!'})
  t=r['workspace'];h={'x-tenant':t,'Authorization':'Bearer '+r['token'],'x-commerce-locale':'de-DE'};owners[label]=h
  call(core_url,'/api/apps',{'manifest':package},h)
  digest=call(core_url,'/api/apps/marketplaces/credentials',h=h)['digest']
  k=call(core_url,'/api/apps/marketplaces/credentials',{'digest':digest,'approve':True,'permissions':['data.read','data.write','products.read','channels.read','orders.read','orders.import','payments.record','customers.pii','jobs.write'],'expiresInDays':1},h)
  tenants[t]={'coreUrl':core_url,'callbackToken':k['key'],'accounts':{'amazon':{'provider':'amazon','clientId':'fixture','clientSecret':'fixture','refreshToken':'fixture','sellerId':'seller'},'ebay':{'provider':'ebay','region':'sandbox','clientId':'fixture','clientSecret':'fixture','refreshToken':'fixture'},'kaufland':{'provider':'kaufland','key':'fixture-key','secret':'fixture-secret'}}}
 def start():
  p=subprocess.Popen([str(ROOT/'target/debug/marketplace-gateway')],env={**env,'APP_TOKEN':gateway,'APP_MARKETPLACE_TENANTS':json.dumps(tenants),'APP_BIND_ADDR':adapter_url.removeprefix('http://'),'MARKETPLACE_ALLOW_FIXTURE':'1','MARKETPLACE_TEST_ORIGIN':seller_url},stdout=log,stderr=log)
  for _ in range(100):
   if p.poll()is not None:raise AssertionError('Marketplace startup failed; see local log')
   try:call(adapter_url,'/health');return p
   except OSError:time.sleep(.05)
  raise AssertionError('Marketplace health unavailable')
 adapter=start();h=owners['market'];t=h['x-tenant'];foreign=owners['market-foreign'];app='/api/apps/marketplaces/actions/'
 def action(name,v,h=h,status=200):return call(core_url,app+name,v,h,status)
 def rows(entity):return action('list_'+entity,{})['elements']
 def save(entity,id,fields,revision=0):return action('save_'+entity,{'id':id,'revision':revision,'fields':fields})
 product=call(core_url,'/api/merchant/products?limit=1',h=h)['elements'][0]['id']
 chdata={'name':{'en-GB':'Marketplace','de-DE':'Marktplatz'},'kind':'marketplace','active':True,'visibility':'public','locales':['en-GB','de-DE'],'productIds':[]}
 for provider,destination,policy in [('amazon','A1PA6795UKMFR9',{'productType':'HOME'}),('ebay','EBAY_DE',{'categoryId':'1','merchantLocationKey':'warehouse','paymentPolicyId':'1','returnPolicyId':'2','fulfillmentPolicyId':'3'}),('kaufland','de',{'ean':'4009750243800','vatIndicator':'standard_rate','shippingGroupId':'1'})]:
  call(core_url,'/api/automation/channels/'+provider,{'revision':0,'data':chdata},h,method='PUT')
  save('connections',provider,{'name':{'en':'Fixture '+provider},'channel_id':provider,'provider':provider,'account':provider,'destination':destination,'locale':'de-DE','currency':'EUR','active':True,'policy':policy})
  save('listings',provider,{'connection_id':provider,'product_id':product,'listing_key':provider+':LAMP','sku':'LAMP','title':{'de':'Marktplatz Lampe'},'description':None,'images':['https://example.test/lamp.jpg'],'attributes':{},'mapping':{},'price':None,'state':'draft','snapshot':None})
  ready=action('prepare',{'listingId':provider});assert ready['publishable'] and ready['mapped']['title']=='Marktplatz Lampe',ready
  j=call(core_url,app+'sync',{'connectionId':provider,'kind':'listing','listingId':provider},{**h,'Idempotency-Key':'listing-'+provider+suffix})
  def finish(id):
   for _ in range(100):
    job=next(j for j in call(core_url,'/api/apps/marketplaces/jobs',h=h)['jobs']if j['id']==id)
    if job['status'] in ['succeeded','failed','uncertain']:assert job['status']=='succeeded',job;return job
    time.sleep(.1)
   raise AssertionError('Job did not finish')
  finish(j['jobId']);assert next(x for x in rows('listings')if x['id']==provider)['state']=='submitted'
  j=call(core_url,app+'sync',{'connectionId':provider,'kind':'orders'},{**h,'Idempotency-Key':'orders-'+provider+suffix});finish(j['jobId'])
 j=call(core_url,app+'sync',{'connectionId':'kaufland','kind':'listing','listingId':'kaufland'},{**h,'Idempotency-Key':'kaufland-update-'+suffix});finish(j['jobId'])
 assert sum(r[0]=='POST' and r[1].startswith('/v2/units') for r in requests)==1
 assert any(r[0]=='PATCH' and r[1].startswith('/v2/units/11') for r in requests)
 assert len(rows('inbox'))==3
 assert any('/orders/2026-01-01/orders' in r[1] for r in requests)
 assert any('/publish' in r[1] for r in requests)
 kau=next(json.loads(r[2])for r in requests if r[0]=='PUT' and '/product-data' in r[1]);assert kau['ean']==['4009750243800'] and kau['attributes']['title']==['Marktplatz Lampe']
 call(core_url,'/store-api/product',{}, {'x-tenant':t,'sw-sales-channel-id':'amazon'},403)
 action('list_connections',{},foreign);assert action('list_connections',{},foreign)['elements']==[]
 action('prepare',{'listingId':'amazon'},foreign,502)
 call(adapter_url,'/actions/prepare',{'listingId':'amazon'},{'x-tenant':t},401)
 print('PASS real leased sync jobs, Amazon current order API, eBay offer lookup/create/publish, Kaufland signed product/unit APIs and localized overrides')
 assert action('review_order',{'inboxId':next(x['id']for x in rows('inbox')if x['connection_id']=='amazon')})['snapshot']['lines'][0]['productId']==product
 inbox=next(x for x in rows('inbox')if x['connection_id']=='amazon');address={'name':'Buyer Fixture','street':'Lane 1','postalCode':'10115','city':'Berlin','country':'DE'}
 snapshot={'salesChannelId':'amazon','externalId':'external-1','currency':'EUR','locale':'de-DE','email':'buyer'+suffix+'@example.test','billingAddress':address,'shippingAddress':address,'lines':[{'productId':product,'quantity':2,'amount':{'grossMinor':2000,'taxMinor':319,'taxRateBasisPoints':1900}}],'shipping':{'grossMinor':0,'taxMinor':0,'taxRateBasisPoints':0},'totalMinor':2000,'paid':True}
 kh={'x-tenant':t,'Authorization':'Bearer '+tenants[t]['callbackToken']}
 stock=call(core_url,'/store-api/product/'+product,h={'x-tenant':t})['product']['stock']
 args={'inboxId':inbox['id'],'snapshot':snapshot,'approve':True}
 for bad in [{**snapshot,'paid':False},{**snapshot,'totalMinor':1999},{**snapshot,'lines':[{'productId':'foreign-product','quantity':1,'amount':{'grossMinor':2000,'taxMinor':319,'taxRateBasisPoints':1900}}]}]:action('import_order',{**args,'snapshot':bad},status=502)
 flow_id='market_order_'+suffix
 names={l:'Marketplace order' for l in ['en','de','fr','es']}
 flow={'name':names,'active':True,'event':'order.placed','condition':{'type':'alwaysValid'},'action':'pipeline','instruction':names,'locale':'de-DE','pipeline':{'entry':'field','nodes':[{'kind':'action','id':'field','action':'action.set.order.custom.field','config':{'field':'marketImported','value':True},'next':None}]}}
 call(core_url,'/api/automation/flows/'+flow_id,{'revision':0,'data':flow},h,method='PUT')
 limited=call(core_url,'/api/workspace/integrations',{'name':'Only order writing','expiresInDays':1,'permissions':['orders.write']},h)
 restricted={**h,'Authorization':'Bearer '+limited['key']}
 action('import_order',args,restricted,403)
 tools=call(core_url,'/mcp',{'jsonrpc':'2.0','id':1,'method':'tools/list'},restricted)['result']['tools']
 assert not any(tool['name']=='app.marketplaces.import_order' for tool in tools)
 result=action('import_order',args);oid=result['orderId'];assert result['nativeOrder']
 for _ in range(150):
  jobs=call(core_url,'/api/automation',h=h)['jobs']
  if any(j['flow']==flow_id and j['state']=='completed' for j in jobs):break
  time.sleep(.1)
 else:raise AssertionError(('Native order.placed Flow did not complete',jobs))
 assert action('import_order',args)['replayed']
 with concurrent.futures.ThreadPoolExecutor(max_workers=4)as pool:assert all(v['orderId']==oid for v in pool.map(lambda _:action('import_order',args),range(4)))
 assert call(core_url,'/store-api/product/'+product,h={'x-tenant':t})['product']['stock']==stock-2
 order=call(core_url,'/api/merchant/orders/'+oid,h=h);assert order['price']['totalMinor']=='2000' and order['payment']['externallyPaid']
 assert order['automation']['customFields']['marketImported'] is True
 assert 'token' not in order['cart'];assert order['lineItems'][0]['label']!=''
 customers=call(core_url,'/api/merchant/customers',h=h);assert snapshot['email'] in json.dumps(customers)
 call(core_url,'/api/merchant/orders/'+oid,h=foreign,status=404)
 call(core_url,'/api/apps/marketplaces/core/order_import',snapshot,{**kh,'x-tenant':foreign['x-tenant']},403)
 stop(adapter);adapter=start();assert action('import_order',args)['replayed']
 assert call(core_url,'/store-api/product/'+product,h={'x-tenant':t})['product']['stock']==stock-2
 seller=call(core_url,'/api/merchant/receipts/settings',h=h)
 call(core_url,'/api/merchant/receipts/settings',{'revision':seller['revision'],'data':{'name':'Synthetic Seller GmbH','address':'Test Street 1, Berlin','taxId':'TEST-ONLY'}},h,method='PUT')
 receipt=call(core_url,'/api/merchant/orders/'+oid+'/receipts',{'revision':order['revision'],'kind':'invoice','locale':'de','requestKey':'external-invoice-'+suffix},h)
 assert call(core_url,'/api/merchant/receipts/'+receipt['id']+'/pdf',h=h).startswith(b'%PDF-1.4')
 verify_order_localization(call,core_url,h,foreign,kh,snapshot,env,ROOT/'artifacts/marketplaces.log')
 ch=call(core_url,'/api/automation',h=h)['channels'];channel=next(c for c in ch if c['id']=='amazon')
 call(core_url,'/api/automation/channels/amazon',{'revision':channel['revision'],'data':{**channel['data'],'active':False}},h,method='PUT')
 action('prepare',{'listingId':'amazon'},status=502)
 print('PASS native paid order, exact source money/tax, guest customer, native order.placed Flow side effect, shared invoice/PDF, paused-channel denial, stock consumed once across concurrent replays and service restart; foreign tenant denied')
finally:
 if adapter:stop(adapter)
 if core:stop(core)
 server.shutdown();log.close()
