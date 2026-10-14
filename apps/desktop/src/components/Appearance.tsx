import { useEffect, useState } from "react";

export type Theme = "sky" | "glass" | "dark";
export const themeNames: Record<Theme, string> = { sky: "晴空", glass: "玻璃", dark: "深空" };
const defaults = {
  sky: { accent: "#2f6bff", bg: "#fafafa" },
  glass: { accent: "#2f6bff", bg: "#ffffff" },
  dark: { accent: "#6f9bff", bg: "#09090b" },
};
export interface Palette { accent: string | null; bg: string | null; blur: number; opacity: number }
export const defaultPalette = (): Palette => ({ accent: null, bg: null, blur: 18, opacity: 55 });

// Direct extraction of the prototype's mixHex / HSL palette math, with TS types.
export function hexToRgb(h: string) {
  h = h.replace("#", "");
  if (h.length === 3) h = h[0]+h[0]+h[1]+h[1]+h[2]+h[2];
  return { r: parseInt(h.substr(0,2),16), g: parseInt(h.substr(2,2),16), b: parseInt(h.substr(4,2),16) };
}
export function rgbToHex(r: number, g: number, b: number) {
  function c(x: number) { x=Math.max(0,Math.min(255,Math.round(x))); const s=x.toString(16); return s.length===1?'0'+s:s; }
  return '#'+c(r)+c(g)+c(b);
}
export function mixHex(a: string, b: string, ratio: number) {
  const A=hexToRgb(a),B=hexToRgb(b);
  return rgbToHex(A.r+(B.r-A.r)*ratio,A.g+(B.g-A.g)*ratio,A.b+(B.b-A.b)*ratio);
}
export function hexToHsl(h: string) {
  const c=hexToRgb(h),r=c.r/255,g=c.g/255,b=c.b/255,mx=Math.max(r,g,b),mn=Math.min(r,g,b),l=(mx+mn)/2;
  let s=0,hh=0;
  if(mx!==mn){const d=mx-mn;s=l>0.5?d/(2-mx-mn):d/(mx+mn);switch(mx){case r:hh=(g-b)/d+(g<b?6:0);break;case g:hh=(b-r)/d+2;break;default:hh=(r-g)/d+4;}hh*=60;}
  return {h:hh,s:s*100,l:l*100};
}
export function hslToHex(h: number,s: number,l: number) {
  h=((h%360)+360)%360/360;s=Math.max(0,Math.min(100,s))/100;l=Math.max(0,Math.min(100,l))/100;
  let r,g,b;
  if(s===0){r=g=b=l;}else{
    function hue(p: number,q: number,t: number){if(t<0)t+=1;if(t>1)t-=1;if(t<1/6)return p+(q-p)*6*t;if(t<1/2)return q;if(t<2/3)return p+(q-p)*(2/3-t)*6;return p;}
    const q=l<0.5?l*(1+s):l+s-l*s,pp=2*l-q;r=hue(pp,q,h+1/3);g=hue(pp,q,h);b=hue(pp,q,h-1/3);
  }
  return rgbToHex(r*255,g*255,b*255);
}
export function paletteStyles(theme: Theme, st: Palette): Record<string, string> {
  const vars: Record<string,string> = {};
  if(st.accent){vars['--blue']=st.accent;vars['--blue-dark']=mixHex(st.accent,'#000000',0.22);vars['--blue-soft']=theme==='dark'?mixHex(st.accent,'#000000',0.55):mixHex(st.accent,'#ffffff',0.88);vars['--accent-line']=mixHex(st.accent,'#ffffff',0.55);}
  if(st.bg){
    if(theme==='glass'){
      const hsl=hexToHsl(st.bg),c1=hslToHex(hsl.h,Math.max(55,hsl.s),72),c2=hslToHex(hsl.h+45,Math.max(55,hsl.s),78),c3=hslToHex(hsl.h-35,Math.max(50,hsl.s),76),c4=hslToHex(hsl.h+85,Math.max(50,hsl.s),80),base1=hslToHex(hsl.h,Math.min(70,hsl.s),92),base2=hslToHex(hsl.h+40,Math.min(60,hsl.s),94);
      function rgba(hex: string,op: number){const cc=hexToRgb(hex);return 'rgba('+cc.r+','+cc.g+','+cc.b+','+op+')';}
      vars['--canvas']='radial-gradient(55% 45% at 12% 6%, '+rgba(c1,0.34)+', rgba(255,255,255,0) 62%), radial-gradient(48% 40% at 88% 10%, '+rgba(c2,0.30)+', rgba(255,255,255,0) 62%), radial-gradient(55% 55% at 85% 92%, '+rgba(c3,0.28)+', rgba(255,255,255,0) 62%), radial-gradient(45% 45% at 15% 88%, '+rgba(c4,0.26)+', rgba(255,255,255,0) 62%), linear-gradient(155deg,'+base1+' 0%,'+base2+' 52%,'+base1+' 100%)';
    }else{vars['--bg']=st.bg;vars['--well']=mixHex(st.bg,theme==='dark'?'#ffffff':'#000000',theme==='dark'?0.06:0.04);}
  }
  if(theme==='glass'){vars['--glass-blur']=st.blur+'px';vars['--card']='rgba(255,255,255,'+(st.opacity/100)+')';}
  return vars;
}
export function useAppearance() {
  const [theme,setTheme]=useState<Theme>('sky');
  const [palettes,setPalettes]=useState<Record<Theme,Palette>>({sky:defaultPalette(),glass:defaultPalette(),dark:defaultPalette()});
  useEffect(()=>{
    const root=document.documentElement;root.dataset.theme=theme;
    const vars=paletteStyles(theme,palettes[theme]);Object.entries(vars).forEach(([key,value])=>root.style.setProperty(key,value));
    return ()=>{Object.keys(vars).forEach(key=>root.style.removeProperty(key));delete root.dataset.theme;};
  },[theme,palettes]);
  return {theme,setTheme,palette:palettes[theme],setPalette:(palette:Palette)=>setPalettes(prev=>({...prev,[theme]:palette}))};
}
export function ThemeSwitch({theme,onChange}:{theme:Theme;onChange:(theme:Theme)=>void}) {
  return <div className="theme-switch" role="group" aria-label="外观主题">{(Object.keys(themeNames) as Theme[]).map(name=><button type="button" key={name} className={theme===name?'active':''} aria-pressed={theme===name} onClick={()=>onChange(name)}>{themeNames[name]}</button>)}</div>;
}
export function Appearance({theme,palette,onChange}:{theme:Theme;palette:Palette;onChange:(value:Palette)=>void}) {
  return <section className="card-box palette" aria-label="主题自定义">
    <h4 className="palette-title">主题自定义 <button type="button" onClick={()=>onChange(defaultPalette())}>恢复默认配色</button></h4>
    <p className="card-box-desc">配色仅在本次会话生效，各主题独立；未写入持久化配置。</p>
    <label>主色 <input aria-label="主色" type="color" value={palette.accent??defaults[theme].accent} onChange={e=>onChange({...palette,accent:e.target.value})}/></label>
    <label>背景色 <input aria-label="背景色" type="color" value={palette.bg??defaults[theme].bg} onChange={e=>onChange({...palette,bg:e.target.value})}/></label>
    {theme==='glass'?<>
      <label>玻璃模糊 <output>{palette.blur}px</output><input aria-label="玻璃模糊" type="range" min="0" max="32" value={palette.blur} onChange={e=>onChange({...palette,blur:Number(e.target.value)})}/></label>
      <label>玻璃不透明度 <output>{palette.opacity}%</output><input aria-label="玻璃不透明度" type="range" min="20" max="90" value={palette.opacity} onChange={e=>onChange({...palette,opacity:Number(e.target.value)})}/></label>
    </>:null}
  </section>;
}
