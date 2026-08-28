const CACHE='run-proof-shell-v3';
const SHELL=['/','/assets/app.js','/assets/app.css','/assets/run-proof-diorama-720.webp','/assets/run-proof-diorama.webp'];
self.addEventListener('install',event=>event.waitUntil(caches.open(CACHE).then(cache=>Promise.all(SHELL.map(async url=>{const response=await fetch(url,{cache:'reload'});if(!response.ok)throw new Error(`Could not refresh ${url}`);await cache.put(url,response);}))).then(()=>self.skipWaiting())));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key!==CACHE).map(key=>caches.delete(key)))).then(()=>self.clients.claim())));
self.addEventListener('fetch',event=>{
  const request=event.request;
  if(request.method!=='GET'||new URL(request.url).origin!==location.origin||new URL(request.url).pathname.startsWith('/api/'))return;
  event.respondWith(fetch(request).then(response=>{if(response.ok){const copy=response.clone();caches.open(CACHE).then(cache=>cache.put(request,copy));}return response;}).catch(()=>caches.match(request).then(hit=>hit||((request.mode==='navigate')?caches.match('/'):undefined))));
});
