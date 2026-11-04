import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright');
const esbuild = require('esbuild');
const desktop = process.cwd();
const evidence = path.resolve(desktop, '../../.pi/development/discover-fix/browser');
let browser, server, base;
beforeAll(async () => {
  fs.mkdirSync(evidence, { recursive: true });
  const entry = `import React from 'react'; import {createRoot} from 'react-dom/client';
    import {DiscoverPanel} from ${JSON.stringify(desktop + '/src/components/DiscoverPanel.tsx')};
    import ${JSON.stringify(desktop + '/src/components/prototype.css')};
    const items = Array.from({length:6},(_,i)=>({id:'owned-'+i,name:i===2?'VeryLongUnbrokenPluginName'.repeat(5):'Owned Plugin '+i,
      version:'1.0.0',author:'Owned Test Author',description:'受控布局夹具，不是真实市场或安装验收。',repo_url:'https://github.com/example/owned-'+i,
      kind:i%2?'external':'managed',category:'proxy',tags:[],protocol:'openai_chat',stars:null,verified:false}));
    window.__catalog=items; createRoot(document.getElementById('root')).render(<div className="layout"><aside className="sidebar">Owned sidebar</aside><main className="main"><DiscoverPanel initialCatalog={items}/></main></div>);`;
  await esbuild.build({ stdin:{contents:entry,resolveDir:desktop,loader:'tsx'},outfile:evidence+'/bundle.js',bundle:true,platform:'browser',nodePaths:[desktop+'/node_modules'],plugins:[{name:'owned-ipc',setup(build){
    build.onResolve({filter:/lib\/tauri$/},()=>({path:'owned',namespace:'owned'}));
    build.onLoad({filter:/.*/,namespace:'owned'},()=>({contents:`export async function invoke(cmd){ window.__calls=(window.__calls||[]).concat(cmd); if(cmd==='get_marketplace_catalog')return {items:window.__catalog,is_offline:true,notice:'无法连接 GitHub 市场：连接超时。'+ 'https://api.github.com/'.repeat(30)}; throw new Error('Owned failure'); }`}));
  }}] });
  server=http.createServer((req,res)=>{const file=req.url==='/bundle.js'?'bundle.js':req.url==='/bundle.css'?'bundle.css':null;res.setHeader('Content-Type',file?(file.endsWith('css')?'text/css':'application/javascript'):'text/html; charset=utf-8');res.end(file?fs.readFileSync(evidence+'/'+file):'<html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="stylesheet" href="/bundle.css"><div id="root"></div><script src="/bundle.js"></script></html>');});
  await new Promise(r=>server.listen(0,'127.0.0.1',r)); base='http://127.0.0.1:'+server.address().port;
  browser=await chromium.launch({headless:true});
},30000);
afterAll(async()=>{await browser?.close(); if(server)await new Promise(r=>server.close(r));});
describe('Discover responsive views (owned browser, not native acceptance)',()=>{
  for(const width of [360,768,1024,1366,1414,1920])it(`fits ${width}px and keeps both views functional`,async()=>{
    const page=await browser.newPage({viewport:{width,height:900}});
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(base);await page.locator('[data-plugin-id]').first().waitFor();
    await page.addStyleTag({content:'*,*::before,*::after{animation:none!important;transition:none!important}'});
    await page.evaluate(()=>document.fonts.ready);
    await page.getByRole('button',{name:'刷新市场',exact:true}).click();await page.locator('#marketplaceNotice').waitFor();
    // Keep the offline error visible while checking long-content layout in both views.
    for(const mode of ['cards','list']){
      await page.getByRole('button',{name:mode==='cards'?'详情卡片':'简易列表',exact:true}).click();
      await expect.poll(()=>page.locator('#marketplaceCardsGrid').getAttribute('data-view')).toBe(mode);
      await page.getByRole('button',{name:mode==='cards'?'详情卡片':'简易列表',exact:true}).focus();
      expect(await page.getByRole('button',{name:mode==='cards'?'详情卡片':'简易列表',exact:true}).getAttribute('aria-pressed')).toBe('true');
      expect(await page.locator('[data-plugin-id]').count()).toBe(6);
      const dimensions=await page.evaluate(()=>{const main=document.querySelector('.main');const panel=document.querySelector('#panel-discover');return {mainClient:main.clientWidth,mainScroll:main.scrollWidth,panelClient:panel.clientWidth,panelScroll:panel.scrollWidth};});
      expect(dimensions.mainScroll,JSON.stringify(dimensions)).toBeLessThanOrEqual(dimensions.mainClient+1);
      expect(dimensions.panelScroll,JSON.stringify(dimensions)).toBeLessThanOrEqual(dimensions.panelClient+1);
      await page.screenshot({path:evidence+`/${width}-${mode}.png`,fullPage:true});
    }
    await page.locator('#marketSearch').fill('owned plugin 1');
    expect(await page.locator('[data-plugin-id]').count()).toBe(1);
    await page.locator('#detailBtn-owned-1').click();expect(await page.locator('#marketplaceDetailModal').isVisible()).toBe(true);
    expect(await page.getByRole('button',{name:'简易列表',exact:true}).getAttribute('aria-pressed')).toBe('true');
    const modalOverflow=await page.locator('#marketplaceDetailModal').evaluate(el=>el.scrollWidth>el.clientWidth+1);
    expect(modalOverflow).toBe(false);
    await page.locator('#marketplaceDetailModal').getByRole('button',{name:'关闭',exact:true}).click();
    await page.getByRole('button',{name:'详情卡片',exact:true}).focus();
    await page.keyboard.press('Enter');
    await expect.poll(()=>page.locator('#marketplaceCardsGrid').getAttribute('data-view')).toBe('cards');
    expect(await page.locator('[data-plugin-id]').count()).toBe(1);
    expect(errors).toEqual([]);await page.close();
  },30000);
});
