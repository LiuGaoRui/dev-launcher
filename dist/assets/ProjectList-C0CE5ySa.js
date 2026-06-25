import{c as M,a as I,b as y,d as G,h as O,u as _e,e as Ne,f as De,g as Oe,i as Te,j as xe,k as H,l as $e,m as ve,r as Q,n as je,o as Fe,p as S,q as Me,s as Ie,t as R,N as Ue,v as Ee,w as he,x as Z,y as ge,z as K,A as Ae,B as k,C as N,D as h,E as Le,F as Ge,G as We,H as Y,I as l,J as T,K as b,L as V,M as d,_ as le,O as ue,P as qe,Q as se,R as E,S as ye,T as He,U as Ke,V as Be,W as Se,X as Ye,Y as we,Z as ke,$ as Je,a0 as Xe,a1 as Qe,a2 as Ze,a3 as et,a4 as tt}from"./index-CON1Pm6Z.js";import{N as X,a as re,b as nt,u as ot}from"./group-DwLGpp5r.js";import{P as rt,S as at,M as lt,a as it}from"./MetricsBar-qlJWi86N.js";import{N as st,a as Ce,u as ut,b as dt,c as ct}from"./use-message-BmUESwx_.js";const pt=M("input-group",`
 display: inline-flex;
 width: 100%;
 flex-wrap: nowrap;
 vertical-align: bottom;
`,[I(">",[M("input",[I("&:not(:last-child)",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `),I("&:not(:first-child)",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 margin-left: -1px!important;
 `)]),M("button",[I("&:not(:last-child)",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `,[y("state-border, border",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `)]),I("&:not(:first-child)",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `,[y("state-border, border",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `)])]),I("*",[I("&:not(:last-child)",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `,[I(">",[M("input",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `),M("base-selection",[M("base-selection-label",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `),M("base-selection-tags",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `),y("box-shadow, border, state-border",`
 border-top-right-radius: 0!important;
 border-bottom-right-radius: 0!important;
 `)])])]),I("&:not(:first-child)",`
 margin-left: -1px!important;
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `,[I(">",[M("input",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `),M("base-selection",[M("base-selection-label",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `),M("base-selection-tags",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `),y("box-shadow, border, state-border",`
 border-top-left-radius: 0!important;
 border-bottom-left-radius: 0!important;
 `)])])])])])]),ft={},mt=G({name:"InputGroup",props:ft,setup(n){const{mergedClsPrefixRef:g}=_e(n);return Ne("-input-group",pt,g),{mergedClsPrefix:g}},render(){const{mergedClsPrefix:n}=this;return O("div",{class:`${n}-input-group`},this.$slots)}});function bt(n){const{primaryColor:g,opacityDisabled:e,borderRadius:f,textColor3:r}=n;return Object.assign(Object.assign({},Oe),{iconColor:r,textColor:"white",loadingColor:g,opacityDisabled:e,railColor:"rgba(0, 0, 0, .14)",railColorActive:g,buttonBoxShadow:"0 1px 4px 0 rgba(0, 0, 0, 0.3), inset 0 0 1px 0 rgba(0, 0, 0, 0.05)",buttonColor:"#FFF",railBorderRadiusSmall:f,railBorderRadiusMedium:f,railBorderRadiusLarge:f,buttonBorderRadiusSmall:f,buttonBorderRadiusMedium:f,buttonBorderRadiusLarge:f,boxShadowFocus:`0 0 0 2px ${Te(g,{alpha:.2})}`})}const vt={common:De,self:bt},ht=M("switch",`
 height: var(--n-height);
 min-width: var(--n-width);
 vertical-align: middle;
 user-select: none;
 -webkit-user-select: none;
 display: inline-flex;
 outline: none;
 justify-content: center;
 align-items: center;
`,[y("children-placeholder",`
 height: var(--n-rail-height);
 display: flex;
 flex-direction: column;
 overflow: hidden;
 pointer-events: none;
 visibility: hidden;
 `),y("rail-placeholder",`
 display: flex;
 flex-wrap: none;
 `),y("button-placeholder",`
 width: calc(1.75 * var(--n-rail-height));
 height: var(--n-rail-height);
 `),M("base-loading",`
 position: absolute;
 top: 50%;
 left: 50%;
 transform: translateX(-50%) translateY(-50%);
 font-size: calc(var(--n-button-width) - 4px);
 color: var(--n-loading-color);
 transition: color .3s var(--n-bezier);
 `,[xe({left:"50%",top:"50%",originalTransform:"translateX(-50%) translateY(-50%)"})]),y("checked, unchecked",`
 transition: color .3s var(--n-bezier);
 color: var(--n-text-color);
 box-sizing: border-box;
 position: absolute;
 white-space: nowrap;
 top: 0;
 bottom: 0;
 display: flex;
 align-items: center;
 line-height: 1;
 `),y("checked",`
 right: 0;
 padding-right: calc(1.25 * var(--n-rail-height) - var(--n-offset));
 `),y("unchecked",`
 left: 0;
 justify-content: flex-end;
 padding-left: calc(1.25 * var(--n-rail-height) - var(--n-offset));
 `),I("&:focus",[y("rail",`
 box-shadow: var(--n-box-shadow-focus);
 `)]),H("round",[y("rail","border-radius: calc(var(--n-rail-height) / 2);",[y("button","border-radius: calc(var(--n-button-height) / 2);")])]),$e("disabled",[$e("icon",[H("rubber-band",[H("pressed",[y("rail",[y("button","max-width: var(--n-button-width-pressed);")])]),y("rail",[I("&:active",[y("button","max-width: var(--n-button-width-pressed);")])]),H("active",[H("pressed",[y("rail",[y("button","left: calc(100% - var(--n-offset) - var(--n-button-width-pressed));")])]),y("rail",[I("&:active",[y("button","left: calc(100% - var(--n-offset) - var(--n-button-width-pressed));")])])])])])]),H("active",[y("rail",[y("button","left: calc(100% - var(--n-button-width) - var(--n-offset))")])]),y("rail",`
 overflow: hidden;
 height: var(--n-rail-height);
 min-width: var(--n-rail-width);
 border-radius: var(--n-rail-border-radius);
 cursor: pointer;
 position: relative;
 transition:
 opacity .3s var(--n-bezier),
 background .3s var(--n-bezier),
 box-shadow .3s var(--n-bezier);
 background-color: var(--n-rail-color);
 `,[y("button-icon",`
 color: var(--n-icon-color);
 transition: color .3s var(--n-bezier);
 font-size: calc(var(--n-button-height) - 4px);
 position: absolute;
 left: 0;
 right: 0;
 top: 0;
 bottom: 0;
 display: flex;
 justify-content: center;
 align-items: center;
 line-height: 1;
 `,[xe()]),y("button",`
 align-items: center; 
 top: var(--n-offset);
 left: var(--n-offset);
 height: var(--n-button-height);
 width: var(--n-button-width-pressed);
 max-width: var(--n-button-width);
 border-radius: var(--n-button-border-radius);
 background-color: var(--n-button-color);
 box-shadow: var(--n-button-box-shadow);
 box-sizing: border-box;
 cursor: inherit;
 content: "";
 position: absolute;
 transition:
 background-color .3s var(--n-bezier),
 left .3s var(--n-bezier),
 opacity .3s var(--n-bezier),
 max-width .3s var(--n-bezier),
 box-shadow .3s var(--n-bezier);
 `)]),H("active",[y("rail","background-color: var(--n-rail-color-active);")]),H("loading",[y("rail",`
 cursor: wait;
 `)]),H("disabled",[y("rail",`
 cursor: not-allowed;
 opacity: .5;
 `)])]),gt=Object.assign(Object.assign({},je.props),{size:String,value:{type:[String,Number,Boolean],default:void 0},loading:Boolean,defaultValue:{type:[String,Number,Boolean],default:!1},disabled:{type:Boolean,default:void 0},round:{type:Boolean,default:!0},"onUpdate:value":[Function,Array],onUpdateValue:[Function,Array],checkedValue:{type:[String,Number,Boolean],default:!0},uncheckedValue:{type:[String,Number,Boolean],default:!1},railStyle:Function,rubberBand:{type:Boolean,default:!0},spinProps:Object,onChange:[Function,Array]});let ae;const yt=G({name:"Switch",props:gt,slots:Object,setup(n){ae===void 0&&(typeof CSS<"u"?typeof CSS.supports<"u"?ae=CSS.supports("width","max(1px)"):ae=!1:ae=!0);const{mergedClsPrefixRef:g,inlineThemeDisabled:e,mergedComponentPropsRef:f}=_e(n),r=je("Switch","-switch",ht,vt,n,g),t=Fe(n,{mergedSize(v){var U,F;if(n.size!==void 0)return n.size;if(v)return v.mergedSize.value;const q=(F=(U=f==null?void 0:f.value)===null||U===void 0?void 0:U.Switch)===null||F===void 0?void 0:F.size;return q||"medium"}}),{mergedSizeRef:x,mergedDisabledRef:w}=t,$=S(n.defaultValue),z=Ae(n,"value"),c=Me(z,$),i=R(()=>c.value===n.checkedValue),s=S(!1),C=S(!1),D=R(()=>{const{railStyle:v}=n;if(v)return v({focused:C.value,checked:i.value})});function A(v){const{"onUpdate:value":U,onChange:F,onUpdateValue:q}=n,{nTriggerFormInput:te,nTriggerFormChange:ne}=t;U&&he(U,v),q&&he(q,v),F&&he(F,v),$.value=v,te(),ne()}function B(){const{nTriggerFormFocus:v}=t;v()}function _(){const{nTriggerFormBlur:v}=t;v()}function u(){n.loading||w.value||(c.value!==n.checkedValue?A(n.checkedValue):A(n.uncheckedValue))}function j(){C.value=!0,B()}function W(){C.value=!1,_(),s.value=!1}function ee(v){n.loading||w.value||v.key===" "&&(c.value!==n.checkedValue?A(n.checkedValue):A(n.uncheckedValue),s.value=!1)}function de(v){n.loading||w.value||v.key===" "&&(v.preventDefault(),s.value=!0)}const ie=R(()=>{const{value:v}=x,{self:{opacityDisabled:U,railColor:F,railColorActive:q,buttonBoxShadow:te,buttonColor:ne,boxShadowFocus:ce,loadingColor:pe,textColor:o,iconColor:a,[Z("buttonHeight",v)]:m,[Z("buttonWidth",v)]:p,[Z("buttonWidthPressed",v)]:L,[Z("railHeight",v)]:P,[Z("railWidth",v)]:oe,[Z("railBorderRadius",v)]:Ve,[Z("buttonBorderRadius",v)]:ze},common:{cubicBezierEaseInOut:Pe}}=r.value;let fe,me,be;return ae?(fe=`calc((${P} - ${m}) / 2)`,me=`max(${P}, ${m})`,be=`max(${oe}, calc(${oe} + ${m} - ${P}))`):(fe=ge((K(P)-K(m))/2),me=ge(Math.max(K(P),K(m))),be=K(P)>K(m)?oe:ge(K(oe)+K(m)-K(P))),{"--n-bezier":Pe,"--n-button-border-radius":ze,"--n-button-box-shadow":te,"--n-button-color":ne,"--n-button-width":p,"--n-button-width-pressed":L,"--n-button-height":m,"--n-height":me,"--n-offset":fe,"--n-opacity-disabled":U,"--n-rail-border-radius":Ve,"--n-rail-color":F,"--n-rail-color-active":q,"--n-rail-height":P,"--n-rail-width":oe,"--n-width":be,"--n-box-shadow-focus":ce,"--n-loading-color":pe,"--n-text-color":o,"--n-icon-color":a}}),J=e?Ie("switch",R(()=>x.value[0]),ie,n):void 0;return{handleClick:u,handleBlur:W,handleFocus:j,handleKeyup:ee,handleKeydown:de,mergedRailStyle:D,pressed:s,mergedClsPrefix:g,mergedValue:c,checked:i,mergedDisabled:w,cssVars:e?void 0:ie,themeClass:J==null?void 0:J.themeClass,onRender:J==null?void 0:J.onRender}},render(){const{mergedClsPrefix:n,mergedDisabled:g,checked:e,mergedRailStyle:f,onRender:r,$slots:t}=this;r==null||r();const{checked:x,unchecked:w,icon:$,"checked-icon":z,"unchecked-icon":c}=t,i=!(ve($)&&ve(z)&&ve(c));return O("div",{role:"switch","aria-checked":e,class:[`${n}-switch`,this.themeClass,i&&`${n}-switch--icon`,e&&`${n}-switch--active`,g&&`${n}-switch--disabled`,this.round&&`${n}-switch--round`,this.loading&&`${n}-switch--loading`,this.pressed&&`${n}-switch--pressed`,this.rubberBand&&`${n}-switch--rubber-band`],tabindex:this.mergedDisabled?void 0:0,style:this.cssVars,onClick:this.handleClick,onFocus:this.handleFocus,onBlur:this.handleBlur,onKeyup:this.handleKeyup,onKeydown:this.handleKeydown},O("div",{class:`${n}-switch__rail`,"aria-hidden":"true",style:f},Q(x,s=>Q(w,C=>s||C?O("div",{"aria-hidden":!0,class:`${n}-switch__children-placeholder`},O("div",{class:`${n}-switch__rail-placeholder`},O("div",{class:`${n}-switch__button-placeholder`}),s),O("div",{class:`${n}-switch__rail-placeholder`},O("div",{class:`${n}-switch__button-placeholder`}),C)):null)),O("div",{class:`${n}-switch__button`},Q($,s=>Q(z,C=>Q(c,D=>O(Ue,null,{default:()=>this.loading?O(Ee,Object.assign({key:"loading",clsPrefix:n,strokeWidth:20},this.spinProps)):this.checked&&(C||s)?O("div",{class:`${n}-switch__button-icon`,key:C?"checked-icon":"icon"},C||s):!this.checked&&(D||s)?O("div",{class:`${n}-switch__button-icon`,key:D?"unchecked-icon":"icon"},D||s):null})))),Q(x,s=>s&&O("div",{key:"checked",class:`${n}-switch__checked`},s)),Q(w,s=>s&&O("div",{key:"unchecked",class:`${n}-switch__unchecked`},s)))))}}),wt={xmlns:"http://www.w3.org/2000/svg","xmlns:xlink":"http://www.w3.org/1999/xlink",viewBox:"0 0 512 512"},kt=G({name:"CheckmarkCircleOutline",render:function(g,e){return k(),N("svg",wt,e[0]||(e[0]=[h("path",{d:"M448 256c0-106-86-192-192-192S64 150 64 256s86 192 192 192s192-86 192-192z",fill:"none",stroke:"currentColor","stroke-miterlimit":"10","stroke-width":"32"},null,-1),h("path",{fill:"none",stroke:"currentColor","stroke-linecap":"round","stroke-linejoin":"round","stroke-width":"32",d:"M352 176L217.6 336L160 272"},null,-1)]))}}),xt={xmlns:"http://www.w3.org/2000/svg","xmlns:xlink":"http://www.w3.org/1999/xlink",viewBox:"0 0 512 512"},$t=G({name:"CloseCircleOutline",render:function(g,e){return k(),N("svg",xt,e[0]||(e[0]=[h("path",{d:"M448 256c0-106-86-192-192-192S64 150 64 256s86 192 192 192s192-86 192-192z",fill:"none",stroke:"currentColor","stroke-miterlimit":"10","stroke-width":"32"},null,-1),h("path",{fill:"none",stroke:"currentColor","stroke-linecap":"round","stroke-linejoin":"round","stroke-width":"32",d:"M320 320L192 192"},null,-1),h("path",{fill:"none",stroke:"currentColor","stroke-linecap":"round","stroke-linejoin":"round","stroke-width":"32",d:"M192 320l128-128"},null,-1)]))}}),Ct={xmlns:"http://www.w3.org/2000/svg","xmlns:xlink":"http://www.w3.org/1999/xlink",viewBox:"0 0 512 512"},_t=G({name:"RefreshOutline",render:function(g,e){return k(),N("svg",Ct,e[0]||(e[0]=[h("path",{d:"M320 146s24.36-12-64-12a160 160 0 1 0 160 160",fill:"none",stroke:"currentColor","stroke-linecap":"round","stroke-miterlimit":"10","stroke-width":"32"},null,-1),h("path",{fill:"none",stroke:"currentColor","stroke-linecap":"round","stroke-linejoin":"round","stroke-width":"32",d:"M256 58l80 80l-80 80"},null,-1)]))}}),Re=Le("build",()=>{const n=S(null),g=S([]),e=S(!1),f=S(null),r=S(null),t=S(""),x=S(null);let w=null,$=null;async function z(C){c(),n.value=C,g.value=[],e.value=!0,f.value=null,r.value=null,t.value="";const D=new Ge;D.onmessage=B=>{B.kind==="exit"?f.value=B.data:g.value.push({kind:B.kind,text:B.data})},x.value=D;const A=new Promise(B=>{$=B});return w=We(C,D),w.then(B=>(r.value=B.duration_ms,e.value=!1,$==null||$(B),$=null,B)).catch(B=>{var _;t.value=typeof B=="string"?B:(_=B==null?void 0:B.message)!=null?_:String(B),e.value=!1,$==null||$(null),$=null}),A}function c(){x.value=null,w=null,$=null,e.value=!1}function i(){c(),n.value=null,g.value=[],f.value=null,r.value=null,t.value=""}function s(){return f.value===0}return{projectId:n,lines:g,running:e,exitCode:f,durationMs:r,error:t,startBuild:z,stopBuild:c,reset:i,isSucceeded:s}}),jt={class:"action-bar"},Bt=G({__name:"ActionBar",props:{running:{type:Boolean},busy:{type:Boolean},canBuild:{type:Boolean}},emits:["start","stop","restart","build","deploy","edit","delete"],setup(n,{emit:g}){const e=g;return(f,r)=>(k(),N("div",jt,[n.running?(k(),Y(l(T),{key:1,size:"tiny",type:"warning",loading:n.busy,onClick:r[1]||(r[1]=t=>e("stop"))},{default:b(()=>[...r[8]||(r[8]=[V(" 停止 ",-1)])]),_:1},8,["loading"])):(k(),Y(l(T),{key:0,size:"tiny",type:"success",loading:n.busy,onClick:r[0]||(r[0]=t=>e("start"))},{default:b(()=>[...r[7]||(r[7]=[V(" 启动 ",-1)])]),_:1},8,["loading"])),d(l(T),{size:"tiny",tertiary:"",disabled:n.busy,onClick:r[2]||(r[2]=t=>e("restart"))},{default:b(()=>[...r[9]||(r[9]=[V("重启",-1)])]),_:1},8,["disabled"]),d(l(T),{size:"tiny",tertiary:"",disabled:n.busy||!n.canBuild,title:n.canBuild?"执行构建命令":"未配置构建命令",onClick:r[3]||(r[3]=t=>e("build"))},{default:b(()=>[...r[10]||(r[10]=[V(" 构建 ",-1)])]),_:1},8,["disabled","title"]),d(l(T),{size:"tiny",type:"primary",tertiary:"",disabled:n.busy||!n.canBuild,title:n.canBuild?"停止 → 构建 → 启动":"未配置构建命令",onClick:r[4]||(r[4]=t=>e("deploy"))},{default:b(()=>[...r[11]||(r[11]=[V(" 发布 ",-1)])]),_:1},8,["disabled","title"]),d(l(T),{size:"tiny",quaternary:"",disabled:n.busy,onClick:r[5]||(r[5]=t=>e("edit"))},{default:b(()=>[...r[12]||(r[12]=[V("编辑",-1)])]),_:1},8,["disabled"]),d(l(T),{size:"tiny",quaternary:"",type:"error",disabled:n.busy||n.running,title:n.running?"请先停止项目再删除":"删除项目",onClick:r[6]||(r[6]=t=>e("delete"))},{default:b(()=>[...r[13]||(r[13]=[V(" 删除 ",-1)])]),_:1},8,["disabled","title"])]))}}),St=le(Bt,[["__scopeId","data-v-beed169f"]]),Rt={class:"card-head"},Vt=["title"],zt={class:"meta"},Pt=["title"],Nt={class:"meta-value ellipsis"},Dt={class:"meta-row"},Ot=["title"],Tt={class:"meta-row"},Ft={class:"meta-value"},Mt={key:0,class:"pid"},It={key:0,class:"metrics-wrap"},Ut=G({__name:"ProjectCard",props:{project:{},status:{},busy:{type:Boolean}},emits:["start","stop","restart","build","deploy","edit","delete","open"],setup(n,{emit:g}){const e=n,f=g,r=R(()=>{var c,i;return(i=(c=e.status)==null?void 0:c.health)!=null?i:"stopped"}),t=R(()=>r.value!=="stopped"),x=R(()=>{var c,i,s;return(s=(i=(c=e.status)==null?void 0:c.pid)!=null?i:e.project.last_pid)!=null?s:null}),w=R(()=>{var c;return!!((c=e.project.build_cmd)!=null&&c.trim())}),$=R(()=>`state-${r.value}`);function z(c){return c.expected_ports.length?c.expected_ports.join(" / "):"-"}return(c,i)=>(k(),N("div",{class:ue(["project-card",$.value]),onClick:i[8]||(i[8]=s=>f("open"))},[h("div",Rt,[d(l(se),{class:"type-icon",size:"16"},{default:b(()=>[d(l(qe))]),_:1}),h("span",{class:"name",title:e.project.name},E(e.project.name),9,Vt),d(l(st),{size:"tiny",type:"primary",bordered:!1},{default:b(()=>[V(E(l(rt)[e.project.type]),1)]),_:1}),d(at,{health:r.value,class:"status-badge"},null,8,["health"])]),h("div",zt,[h("div",{class:"meta-row",title:e.project.path},[i[9]||(i[9]=h("span",{class:"meta-label"},"路径",-1)),h("span",Nt,E(e.project.path||"-"),1)],8,Pt),h("div",Dt,[i[10]||(i[10]=h("span",{class:"meta-label"},"启动",-1)),h("span",{class:"meta-value code",title:e.project.start_cmd},E(e.project.start_cmd||"-"),9,Ot)]),h("div",Tt,[i[11]||(i[11]=h("span",{class:"meta-label"},"端口",-1)),h("span",Ft,E(z(e.project)),1),x.value?(k(),N("span",Mt,"PID "+E(x.value),1)):ye("",!0)])]),e.status&&t.value?(k(),N("div",It,[d(lt,{status:e.status},null,8,["status"])])):ye("",!0),h("div",{class:"actions",onClick:i[7]||(i[7]=He(()=>{},["stop"]))},[d(St,{running:t.value,busy:e.busy,"can-build":w.value,onStart:i[0]||(i[0]=s=>f("start")),onStop:i[1]||(i[1]=s=>f("stop")),onRestart:i[2]||(i[2]=s=>f("restart")),onBuild:i[3]||(i[3]=s=>f("build")),onDeploy:i[4]||(i[4]=s=>f("deploy")),onEdit:i[5]||(i[5]=s=>f("edit")),onDelete:i[6]||(i[6]=s=>f("delete"))},null,8,["running","busy","can-build"])])],2))}}),Et=le(Ut,[["__scopeId","data-v-44537a39"]]);async function At(n={}){return typeof n=="object"&&Object.freeze(n),await Ke("plugin:dialog|open",{options:n})}const Lt={class:"footer"},Gt=G({__name:"ProjectFormDialog",props:{modelValue:{type:Boolean},project:{},groups:{},defaultGroupId:{},submitting:{type:Boolean}},emits:["update:modelValue","submit"],setup(n,{emit:g}){const e=n,f=g,r={name:"",group_id:null,type:"custom",path:"",start_cmd:"",build_cmd:"",expected_ports:"",enabled:!0},t=Ye({...r}),x=S(null),w=R(()=>!!e.project),$=R(()=>w.value?"编辑项目":"新建项目"),z=R({get:()=>e.modelValue,set:_=>f("update:modelValue",_)}),c=it.map(_=>({label:_.label,value:_.value})),i=R(()=>e.groups.map(_=>({label:_.name,value:_.id})));Be(()=>e.modelValue,_=>{var u,j,W;_&&(e.project?(t.name=e.project.name,t.group_id=e.project.group_id,t.type=e.project.type,t.path=e.project.path,t.start_cmd=e.project.start_cmd,t.build_cmd=(u=e.project.build_cmd)!=null?u:"",t.expected_ports=e.project.expected_ports.join(", "),t.enabled=e.project.enabled):(Object.assign(t,{...r}),t.group_id=(j=e.defaultGroupId)!=null?j:null),(W=x.value)==null||W.restoreValidation())});const s={name:[{required:!0,message:"请输入项目名称",trigger:"blur"}],type:[{required:!0,message:"请选择项目类型",trigger:"change"}],path:[{required:!0,message:"请选择项目目录",trigger:"change"}],start_cmd:[{required:!0,message:"请输入启动命令",trigger:"blur"}]};async function C(){var _;try{const u=await At({directory:!0,multiple:!1,title:"选择项目目录",defaultPath:t.path||void 0});typeof u=="string"&&u.length>0&&(t.path=u,(_=x.value)==null||_.restoreValidation())}catch(u){console.debug("pick directory canceled or failed:",u)}}function D(){const _=t.expected_ports.split(/[,，\s]+/).map(u=>u.trim()).filter(u=>u.length>0);return{name:t.name.trim(),group_id:t.group_id,type:t.type,path:t.path.trim(),start_cmd:t.start_cmd.trim(),build_cmd:t.build_cmd.trim()||null,expected_ports:_,enabled:t.enabled}}async function A(){x.value&&await x.value.validate(async _=>{var u;_||f("submit",D(),(u=e.project)!=null?u:null)})}function B(){f("update:modelValue",!1)}return(_,u)=>(k(),Y(l(Se),{show:z.value,"onUpdate:show":u[8]||(u[8]=j=>z.value=j),preset:"card",title:$.value,style:{width:"560px"},"mask-closable":!1},{footer:b(()=>[h("div",Lt,[d(l(T),{onClick:B},{default:b(()=>[...u[11]||(u[11]=[V("取消",-1)])]),_:1}),d(l(T),{type:"primary",loading:e.submitting,onClick:A},{default:b(()=>[V(E(w.value?"保存":"创建"),1)]),_:1},8,["loading"])])]),default:b(()=>[d(l(nt),{ref_key:"formRef",ref:x,model:t,rules:s,"label-width":"88","label-placement":"left","require-mark-placement":"right-hanging"},{default:b(()=>[d(l(X),{label:"项目名称",path:"name"},{default:b(()=>[d(l(re),{value:t.name,"onUpdate:value":u[0]||(u[0]=j=>t.name=j),placeholder:"如：HR后端",clearable:""},null,8,["value"])]),_:1}),d(l(X),{label:"项目类型",path:"type"},{default:b(()=>[d(l(Ce),{value:t.type,"onUpdate:value":u[1]||(u[1]=j=>t.type=j),options:l(c),placeholder:"选择类型"},null,8,["value","options"])]),_:1}),d(l(X),{label:"所属分组",path:"group_id"},{default:b(()=>[d(l(Ce),{value:t.group_id,"onUpdate:value":u[2]||(u[2]=j=>t.group_id=j),options:i.value,placeholder:"不分组",clearable:""},null,8,["value","options"])]),_:1}),d(l(X),{label:"项目目录",path:"path"},{default:b(()=>[d(l(mt),null,{default:b(()=>[d(l(re),{value:t.path,"onUpdate:value":u[3]||(u[3]=j=>t.path=j),placeholder:"点击右侧按钮选择目录",readonly:"",style:{flex:"1"}},null,8,["value"]),d(l(T),{onClick:C},{default:b(()=>[...u[9]||(u[9]=[V("选择...",-1)])]),_:1})]),_:1})]),_:1}),d(l(X),{label:"启动命令",path:"start_cmd"},{default:b(()=>[d(l(re),{value:t.start_cmd,"onUpdate:value":u[4]||(u[4]=j=>t.start_cmd=j),placeholder:"如：npm run dev / mvn spring-boot:run",clearable:""},null,8,["value"])]),_:1}),d(l(X),{label:"构建命令",path:"build_cmd"},{default:b(()=>[d(l(re),{value:t.build_cmd,"onUpdate:value":u[5]||(u[5]=j=>t.build_cmd=j),placeholder:"可选，如：mvn clean package / npm run build",clearable:""},null,8,["value"])]),_:1}),d(l(X),{label:"预期端口",path:"expected_ports"},{default:b(()=>[d(l(re),{value:t.expected_ports,"onUpdate:value":u[6]||(u[6]=j=>t.expected_ports=j),placeholder:"多个端口用逗号分隔，如：8080, 5173",clearable:""},null,8,["value"])]),_:1}),d(l(X),{label:"启用"},{default:b(()=>[d(l(yt),{value:t.enabled,"onUpdate:value":u[7]||(u[7]=j=>t.enabled=j)},null,8,["value"]),u[10]||(u[10]=h("span",{class:"hint"},"关闭后该项目不在列表执行批量操作",-1))]),_:1})]),_:1},8,["model"])]),_:1},8,["show","title"]))}}),Wt=le(Gt,[["__scopeId","data-v-0b305d6c"]]),qt={key:0,class:"placeholder"},Ht={class:"footer"},Kt=G({__name:"BuildDialog",props:{modelValue:{type:Boolean},projectName:{}},emits:["update:modelValue"],setup(n,{emit:g}){const e=n,f=g,r=Re(),t=S(null),x=R({get:()=>e.modelValue,set:c=>f("update:modelValue",c)}),w=R(()=>r.error?"error":r.running?"running":r.exitCode===0?"succeeded":"failed"),$=R(()=>{var c,i;switch(w.value){case"running":return"构建中…";case"succeeded":return`构建成功（退出码 0，耗时 ${(c=r.durationMs)!=null?c:0}ms）`;case"failed":return`构建失败（退出码 ${r.exitCode}，耗时 ${(i=r.durationMs)!=null?i:0}ms）`;case"error":return r.error}});Be(()=>r.lines.length,async()=>{await Je();const c=t.value;c&&(c.scrollTop=c.scrollHeight)});function z(){r.reset()}return(c,i)=>(k(),Y(l(Se),{show:x.value,"onUpdate:show":i[1]||(i[1]=s=>x.value=s),preset:"card",title:`构建「${n.projectName}」`,style:{width:"80vw","max-width":"1100px"},"mask-closable":!1,onAfterLeave:z},{footer:b(()=>[h("div",Ht,[h("div",{class:ue(["status",w.value])},[w.value==="running"?(k(),Y(l(se),{key:0,class:"spin"},{default:b(()=>[d(l(_t))]),_:1})):w.value==="succeeded"?(k(),Y(l(se),{key:1},{default:b(()=>[d(l(kt))]),_:1})):(k(),Y(l(se),{key:2},{default:b(()=>[d(l($t))]),_:1})),h("span",null,E($.value),1)],2),d(l(T),{disabled:l(r).running,onClick:i[0]||(i[0]=s=>x.value=!1)},{default:b(()=>[...i[2]||(i[2]=[V("关闭",-1)])]),_:1},8,["disabled"])])]),default:b(()=>[h("div",{class:"build-output",ref_key:"outBox",ref:t},[(k(!0),N(ke,null,we(l(r).lines,(s,C)=>(k(),N("span",{key:C,class:ue(["line",{err:s.kind==="stderr"}])},E(s.text),3))),128)),!l(r).lines.length&&!l(r).error?(k(),N("span",qt," （等待输出…） ")):ye("",!0)],512)]),_:1},8,["show","title"]))}}),Yt=le(Kt,[["__scopeId","data-v-75be4939"]]),Jt={class:"project-list-page"},Xt={class:"toolbar"},Qt={class:"toolbar-title"},Zt={class:"page-count"},en={class:"toolbar-actions"},tn={class:"group-filter"},nn=["onClick"],on={class:"seg-count"},rn={key:0,class:"grid-loading"},an={key:1,class:"grid"},ln=G({__name:"ProjectList",setup(n){const g=ot(),e=Xe(),f=Re(),r=tt(),t=ut(),x=dt(),w=S(null),$=R(()=>[{id:null,name:"全部"},{id:-1,name:"未分组"}].concat(g.groups.map(a=>({id:a.id,name:a.name}))));function z(o){return o===null?e.projects:o===-1?e.projects.filter(a=>a.group_id===null):e.projects.filter(a=>a.group_id===o)}const c=R(()=>z(w.value));function i(o){return o===null?e.projects.length:z(o).length}const s=S(new Set);function C(o,a){a?s.value.add(o):s.value.delete(o),s.value=new Set(s.value)}async function D(o,a){C(o,!0);const m=await e.safe(a);return C(o,!1),m}async function A(){await Promise.all([g.fetchAll(),e.fetchAll()])}Qe(async()=>{await A(),e.startPolling()}),Ze(()=>{e.stopPolling(),f.reset()});function B(o){r.push({name:"ProjectDetail",params:{id:o.id}})}async function _(o){var m,p;const[,a]=await D(o.id,()=>e.start(o.id));a?t.error(`启动失败：${a}`):t.success(`「${o.name}」已启动（PID ${(p=(m=e.projects.find(L=>L.id===o.id))==null?void 0:m.last_pid)!=null?p:"-"}）`)}async function u(o){const[,a]=await D(o.id,()=>e.stop(o.id));a?t.error(`停止失败：${a}`):t.success(`「${o.name}」已停止`)}async function j(o){const[,a]=await D(o.id,()=>e.restart(o.id));a?t.error(`重启失败：${a}`):t.success(`「${o.name}」已重启`)}const W=S(!1),ee=S("");async function de(o){var a;if(!((a=o.build_cmd)!=null&&a.trim())){t.warning("该项目未配置构建命令");return}ee.value=o.name,W.value=!0,C(o.id,!0);try{await f.startBuild(o.id)}finally{C(o.id,!1)}}async function ie(o){var a;if(!((a=o.build_cmd)!=null&&a.trim())){t.warning("该项目未配置构建命令");return}x.warning({title:"一键发布",content:`确定一键发布「${o.name}」吗？将执行：停止 → 构建 → 启动。`,positiveText:"发布",negativeText:"取消",onPositiveClick:async()=>{ee.value=o.name,W.value=!0,C(o.id,!0);try{if(e.isRunning(o.id)){const[,L]=await e.safe(()=>e.stop(o.id));if(L){t.error(`停止失败，已中止发布：${L}`);return}}const m=await f.startBuild(o.id);if(!m||m.exit_code!==0){t.error(`构建失败（退出码 ${f.exitCode}），已中止发布`);return}const[,p]=await e.safe(()=>e.start(o.id));if(p){t.error(`构建成功但启动失败：${p}`);return}t.success(`「${o.name}」发布完成`)}finally{C(o.id,!1)}}})}async function J(o){if(e.isRunning(o.id)){t.warning("项目运行中，请先停止再删除");return}x.warning({title:"删除确认",content:`确定删除项目「${o.name}」吗？此操作不可恢复。`,positiveText:"删除",negativeText:"取消",onPositiveClick:async()=>{const[,a]=await e.safe(()=>e.remove(o.id));a?t.error(`删除失败：${a}`):t.success(`已删除「${o.name}」`)}})}const v=S(!1),U=S(null),F=S(!1);function q(){U.value=null,v.value=!0}function te(o){U.value=o,v.value=!0}async function ne(o,a){if(F.value=!0,a){const[,m]=await e.safe(()=>e.patch(a.id,o));if(F.value=!1,m){t.error(`保存失败：${m}`);return}t.success(`已更新「${o.name}」`)}else{const[m,p]=await e.safe(()=>e.add(o));if(F.value=!1,p||!m){t.error(`创建失败：${p}`);return}t.success(`已创建「${m.name}」`)}v.value=!1}async function ce(){const o=c.value.filter(m=>!e.isRunning(m.id));if(!o.length){t.info("没有可启动的项目");return}let a=0;for(const m of o){const[,p]=await e.safe(()=>e.start(m.id));p?t.error(`「${m.name}」启动失败：${p}`):a++}await e.probeNow(),a===o.length?t.success(`已启动 ${a} 个项目`):t.warning(`已启动 ${a}/${o.length} 个项目`)}async function pe(){const o=c.value.filter(m=>e.isRunning(m.id));if(!o.length){t.info("没有运行中的项目");return}let a=0;for(const m of o){const[,p]=await e.safe(()=>e.stop(m.id));p?t.error(`「${m.name}」停止失败：${p}`):a++}await e.probeNow(),a===o.length?t.success(`已停止 ${a} 个项目`):t.warning(`已停止 ${a}/${o.length} 个项目`)}return(o,a)=>{const m=et("NSpin");return k(),N("div",Jt,[h("div",Xt,[h("div",Qt,[a[3]||(a[3]=h("span",{class:"page-name"},"项目",-1)),h("span",Zt,E(l(e).projects.length),1)]),h("div",en,[d(l(T),{size:"small",secondary:"",onClick:ce},{default:b(()=>[...a[4]||(a[4]=[V("全部启动",-1)])]),_:1}),d(l(T),{size:"small",secondary:"",onClick:pe},{default:b(()=>[...a[5]||(a[5]=[V("全部停止",-1)])]),_:1}),d(l(T),{size:"small",secondary:"",onClick:a[0]||(a[0]=p=>l(e).probeNow())},{default:b(()=>[...a[6]||(a[6]=[V("刷新",-1)])]),_:1}),d(l(T),{size:"small",type:"primary",onClick:q},{default:b(()=>[...a[7]||(a[7]=[V("+ 新建项目",-1)])]),_:1})])]),h("div",tn,[(k(!0),N(ke,null,we($.value,p=>(k(),N("button",{key:String(p.id),class:ue(["seg-btn",{active:w.value===p.id}]),onClick:L=>w.value=p.id},[V(E(p.name)+" ",1),h("span",on,E(i(p.id)),1)],10,nn))),128))]),l(e).loading?(k(),N("div",rn,[d(m,{size:"small"})])):c.value.length?(k(),N("div",an,[(k(!0),N(ke,null,we(c.value,p=>{var L;return k(),Y(Et,{key:p.id,project:p,status:(L=l(e).statuses[p.id])!=null?L:null,busy:s.value.has(p.id),onStart:P=>_(p),onStop:P=>u(p),onRestart:P=>j(p),onBuild:P=>de(p),onDeploy:P=>ie(p),onEdit:P=>te(p),onDelete:P=>J(p),onOpen:P=>B(p)},null,8,["project","status","busy","onStart","onStop","onRestart","onBuild","onDeploy","onEdit","onDelete","onOpen"])}),128))])):(k(),Y(l(ct),{key:2,description:"还没有项目，点击右上角「新建项目」开始",class:"empty-state"})),d(Wt,{modelValue:v.value,"onUpdate:modelValue":a[1]||(a[1]=p=>v.value=p),project:U.value,groups:l(g).groups,"default-group-id":w.value&&w.value>0?w.value:null,submitting:F.value,onSubmit:ne},null,8,["modelValue","project","groups","default-group-id","submitting"]),d(Yt,{modelValue:W.value,"onUpdate:modelValue":a[2]||(a[2]=p=>W.value=p),"project-name":ee.value},null,8,["modelValue","project-name"])])}}}),pn=le(ln,[["__scopeId","data-v-65ada9f9"]]);export{pn as default};
