import { beforeAll, afterAll, beforeEach, describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright');
const esbuild = require('esbuild');
const desktop = process.cwd();
const evidence = path.resolve(desktop, '../../.pi/development/protocol-capability-alignment/checkpoint-b/browser');
let browser, server, base, page;
const ids = ['claude-code','claude-desktop','codex','dsh','opencode','pi','grok-build','zcode','kimi-code','openclaw','hermes'];
beforeAll(async () => {
  fs.mkdirSync(evidence, { recursive: true });
  fs.writeFileSync(evidence+'/entry.tsx', `
    import React from 'react'; import {createRoot} from 'react-dom/client';
    import {IdePanel} from '${desktop.replaceAll('\\','/')}/src/components/IdePanel';
    window.calls=[]; window.queue=[]; window.copies=[]; window.mode='ok'; window.rejectDetect=false;
    window.targets=${JSON.stringify(ids)}.map((id,i)=>({id,label:id,installed:true,configured:false,supported:![1,2,6].includes(i),protocol:[0,1].includes(i)?'Anthropic Messages':[2,6].includes(i)?'OpenAI Responses':'OpenAI Chat',config_path:'owned/'+id,version:null,note:'owned fixture'}));
    window.capabilities={ingress:[{protocol:'openai_chat',endpoints:['/v1/chat/completions']},{protocol:'anthropic_messages',endpoints:['/v1/messages']},{protocol:'openai_responses',endpoints:['/v1/responses','/responses']}],backend_protocols:['openai_chat','openai_responses','anthropic_messages'],ir_source:'TiyGate 6db40e54663455fb99d675bd8985e4dc50f68a14 / Apache-2.0'};
    window.ipc=async(cmd,args)=>{window.calls.push({cmd,args});
      if(cmd==='detect_agent_clients'){if(window.rejectDetect)throw Error('reject');return window.targets;}
      if(cmd==='flight_list_models'){if(window.rejectModels)throw Error('owned models rejected');return {models:window.models??[{id:'owned-model'}]};}
      if(cmd==='list_agent_backups'){if(window.mode==='reject')throw Error('reject');return [{id:'1775000000000',created_at:1775000000000,path:'owned/backup'}];}
      if(cmd==='import_agent_client'||cmd==='restore_agent_client'){
        if(window.mode==='pending')return new Promise((resolve,reject)=>window.queue.push({resolve,reject}));
        if(window.mode==='reject')throw Error('reject');
        return {models:args.models??[], restored_files:1};
      } throw Error('Unexpected IPC '+cmd);
    };
    Object.defineProperty(navigator,'clipboard',{value:{writeText:text=>{window.copies.push(text);return new Promise((resolve,reject)=>window.queue.push({resolve,reject}));}}, configurable:true});
    window.settle=ok=>{const p=window.queue.shift();if(!p)throw Error('No pending');ok?p.resolve({models:['owned-model'],restored_files:1}):p.reject(Error('owned reject'));};
    let root=createRoot(document.getElementById('root'));
    window.mount=(running=false,key=null,port=28319)=>{if(!root)root=createRoot(document.getElementById('root'));root.render(<IdePanel gatewayStatus={{is_running:running,port,master_key_preview:'owned-preview',protocol_capabilities:window.capabilities}} masterKey={key}/>);};
    window.unmount=()=>{root.unmount();root=null;};window.mount();
  `);
  await esbuild.build({entryPoints:[evidence+'/entry.tsx'],outfile:evidence+'/bundle.js',bundle:true,platform:'browser',jsx:'automatic',nodePaths:[desktop+'/node_modules'],loader:{'.css':'empty','.svg':'dataurl','.png':'dataurl'},plugins:[{name:'owned-ipc',setup(b){b.onResolve({filter:/lib\/tauri$/},()=>({path:'ipc',namespace:'owned'}));b.onLoad({filter:/.*/,namespace:'owned'},()=>({contents:'export const invoke=(cmd,args)=>window.ipc(cmd,args); export class Channel {}'}));}}]});
  const distDir = path.resolve(desktop, 'dist/assets');
  let cssName = null;
  if (fs.existsSync(distDir)) {
    cssName = fs.readdirSync(distDir).find(n => n.endsWith('.css'));
  }
  if (cssName) {
    fs.copyFileSync(distDir + '/' + cssName, evidence + '/tested-built.css');
  } else {
    fs.writeFileSync(evidence + '/tested-built.css', '/* fallback empty styles */');
  }
  server=http.createServer((req,res)=>{if(req.url==='/bundle.js'){res.setHeader('content-type','application/javascript');res.end(fs.readFileSync(evidence+'/bundle.js'));}else if(req.url==='/style.css'){res.setHeader('content-type','text/css');res.end(fs.readFileSync(evidence+'/tested-built.css'));}else{res.setHeader('content-type','text/html; charset=utf-8');res.end('<!doctype html><html data-theme="light"><link rel="stylesheet" href="/style.css"><body><div id="root"></div><script src="/bundle.js"></script></body></html>');}});
  await new Promise(r=>server.listen(0,'127.0.0.1',r));base='http://127.0.0.1:'+server.address().port;browser=await chromium.launch({headless:true});
},30000);
afterAll(async()=>{await browser?.close();if(server)await new Promise(r=>server.close(r));});
beforeEach(async()=>{await page?.close();page=await browser.newPage({viewport:{width:1366,height:950}});await page.goto(base);await page.waitForFunction(()=>document.querySelectorAll('[data-agent-target]').length===11);});
const card=id=>page.locator(`[data-agent-target="${id}"]`);
const feedback=()=>page.locator('[data-agent-feedback]').innerText();
const running=async()=>{await page.evaluate(()=>window.mount(true,'owned-fixture-key'));await page.getByRole('button',{name:'一键接入',exact:true}).first().waitFor();await page.waitForFunction(()=>!document.querySelector('[data-agent-import="opencode"]').disabled);};
describe('real extracted workbench owned browser seam (not native acceptance)',()=>{
  it('renders 11 cards and 4 stats while OFF and keeps client mapping guards separate', async()=>{
    expect(await page.locator('[data-agent-stat]').count()).toBe(4);
    expect(await page.locator('[data-agent-target]').count()).toBe(11);
    expect(await page.locator('body').innerText()).not.toContain('当前统一网关仅支持 OpenAI Chat');
    for(const id of ['claude-desktop','codex','grok-build']){
      expect(await card(id).innerText()).not.toContain('协议不支持');
    }
  });
  it('OFF/null key mounts detection, 11 cards, 4 stats, source layout and real loaded 32px source images',async()=>{
    const calls=await page.evaluate(()=>window.calls.map(c=>c.cmd));expect(calls).toEqual(['detect_agent_clients']);
    expect(await page.locator('[data-agent-stat]').count()).toBe(4);
    for(const text of ['智能体工作台','分发模型配置','全部客户端','网关未启动'])expect(await page.locator('body').innerText()).toContain(text);
    expect(await page.getByRole('button',{name:'一键更新所有已安装智能体'}).isDisabled()).toBe(true);
    await page.waitForFunction(()=>[...document.querySelectorAll('[data-agent-target] img')].every(i=>i.complete&&i.naturalWidth>0));
    const images=await page.locator('[data-agent-target] img').evaluateAll(list=>list.map(i=>({width:i.getBoundingClientRect().width,natural:i.naturalWidth})));
    expect(images).toHaveLength(11);expect(images.every(i=>i.width===32&&i.natural>0)).toBe(true);
    const positions=await page.locator('[data-agent-target]').evaluateAll(list=>list.slice(0,3).map(e=>({x:e.getBoundingClientRect().x,y:e.getBoundingClientRect().y})));
    expect(new Set(positions.map(p=>p.x)).size).toBe(3);expect(new Set(positions.map(p=>p.y)).size).toBe(1);
    await page.screenshot({path:evidence+'/off-light.png',fullPage:true});
    await page.evaluate(()=>document.documentElement.dataset.theme='dark');await page.screenshot({path:evidence+'/off-dark.png',fullPage:true});
  });
  it('detect rejection is visible, statistics unknown rather than zero; retry restores real targets',async()=>{
    await page.evaluate(()=>{window.rejectDetect=true;});await page.getByRole('button',{name:'刷新状态'}).click();
    await page.getByRole('button',{name:'重试检测'}).waitFor();expect(await page.locator('[data-agent-target]').count()).toBe(0);
    expect(await page.locator('[data-agent-stat="本机已安装"]').innerText()).toContain('检测失败');
    expect(await page.locator('body').innerText()).toContain('检测 IPC 被拒绝');
    await page.evaluate(()=>window.rejectDetect=false);await page.getByRole('button',{name:'重试检测'}).click();await page.waitForFunction(()=>document.querySelectorAll('[data-agent-target]').length===11);
  });
  it('path-copy success waits for promise; rejection/retry never leaves stale success',async()=>{
    const copy=card('pi').getByTitle('复制文件路径');await copy.click();expect(await page.locator('[data-agent-feedback]').count()).toBe(0);
    await page.evaluate(()=>window.settle(false));expect(await feedback()).toContain('复制失败');expect(await feedback()).not.toContain('已复制');
    await copy.click();await page.evaluate(()=>window.settle(true));expect(await feedback()).toContain('已复制 配置路径');
    await copy.click();expect(await page.locator('[data-agent-feedback]').count()).toBe(0);await page.evaluate(()=>window.settle(false));expect(await feedback()).not.toContain('已复制');
    expect(await page.evaluate(()=>window.copies)).toEqual(['owned/pi','owned/pi','owned/pi']);
  });
  it.each([true,false])('ignores clipboard settlement %s after unmount',async ok=>{
    await card('pi').getByTitle('复制文件路径').click();await page.evaluate(()=>{window.unmount();window.mount();});await page.waitForFunction(()=>document.querySelectorAll('[data-agent-target]').length===11);await page.evaluate(ok=>window.settle(ok),ok);expect(await page.locator('[data-agent-feedback]').count()).toBe(0);
  });
  it('per-target lock prevents same-target overlap; rejected IPC visible',async()=>{
    await running();
    expect(await card('claude-code').getByRole('button',{name:'一键接入',exact:true}).isEnabled()).toBe(true);
    await page.evaluate(()=>window.mode='pending');const write=card('opencode').getByRole('button',{name:'一键接入',exact:true});await write.click();await write.evaluate(b=>b.click());
    expect((await page.evaluate(()=>window.calls)).filter(c=>c.cmd==='import_agent_client')).toHaveLength(1);
    expect(await card('pi').getByRole('button',{name:'一键接入',exact:true}).isEnabled()).toBe(true);
    expect(await page.getByRole('button',{name:'一键更新所有已安装智能体'}).isDisabled()).toBe(true);
    await page.evaluate(()=>window.settle(false));expect(await feedback()).toContain('失败');expect(await feedback()).not.toContain('已成功');
    await page.evaluate(()=>window.mode='ok');await write.click();await page.waitForFunction(()=>document.querySelector('[data-agent-feedback]')?.textContent.includes('已成功更新'));
  });
  it('backup list/restore rejection, ms display, explicit restore and batch protocol filter',async()=>{
    await page.evaluate(()=>window.mode='reject');await card('pi').getByRole('button',{name:'备份历史'}).click();await page.getByRole('button',{name:'重试读取备份'}).waitFor();expect(await page.getByRole('dialog').innerText()).toContain('备份 IPC 被拒绝');
    await page.evaluate(()=>window.mode='ok');await page.getByRole('button',{name:'重试读取备份'}).click();await page.getByRole('button',{name:'恢复此版本'}).waitFor();expect(await page.getByRole('dialog').innerText()).toContain('2026');
    expect((await page.evaluate(()=>window.calls)).some(c=>c.cmd==='restore_agent_client')).toBe(false);
    await page.evaluate(()=>window.mode='reject');await page.getByRole('button',{name:'恢复此版本'}).click();expect(await feedback()).toContain('恢复 IPC 被拒绝');
    await page.getByRole('button',{name:'关闭',exact:true}).click();await running();await page.evaluate(()=>{window.mode='ok';window.calls=[];});await page.getByRole('button',{name:'一键更新所有已安装智能体'}).click();await page.waitForFunction(()=>document.querySelector('[data-agent-feedback]')?.textContent.includes('一键更新'));
  });
  it('Claude Code writes only after IPC settles',async()=>{
    await running();
    const write=card('claude-code').getByRole('button',{name:'一键接入',exact:true});
    await page.evaluate(()=>window.mode='pending');
    await write.click();
    expect(await page.locator('[data-agent-feedback]').count()).toBe(0);
    await page.evaluate(()=>window.settle(false));await page.waitForFunction(()=>document.querySelector('[data-agent-feedback]')?.textContent.includes('失败'));
    expect(await feedback()).not.toContain('已成功');
    await write.click();await page.evaluate(()=>window.settle(true));await page.waitForFunction(()=>document.querySelector('[data-agent-feedback]')?.textContent.includes('已成功更新'));
    expect((await page.evaluate(()=>window.calls)).filter(c=>c.cmd==='import_agent_client').map(c=>c.args)).toEqual([{target:'claude-code',models:['owned-model']},{target:'claude-code',models:['owned-model']}]);
  });
  it.each(['off','key','not-installed'])('does not write when %s prerequisite is absent',async absent=>{
    await page.evaluate(absent=>{
      if(absent==='not-installed')window.targets.find(t=>t.id==='claude-code').installed=false;
      window.calls=[];window.mount(absent!=='off',absent==='key'?null:'owned-fixture-key',28319);
    },absent);
    if(absent==='not-installed')await page.getByRole('button',{name:'刷新状态'}).click();
    await page.waitForFunction(()=>document.querySelector('[data-agent-import="claude-code"]').disabled);
    await card('claude-code').locator('[data-agent-import]').evaluate(b=>b.click());
    expect((await page.evaluate(()=>window.calls)).filter(c=>c.cmd==='import_agent_client')).toHaveLength(0);
  });
});
