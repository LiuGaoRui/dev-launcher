import{c as L,a as w,k as q,b as E,d as O,h as B,u as Q,n as J,s as X,t as P,a5 as W,a6 as ee,a7 as te,A as ae,p as _,a1 as Y,a8 as se,a9 as oe,aa as re,ab as ne,r as le,U as V,E as ie,F as ce,ac as j,a0 as ue,ad as de,a2 as ve,V as fe,C as I,D as n,M as g,K as C,I as d,H as F,Z as me,R,S as M,J as U,B as $,L as S,a4 as he,ae as pe,$ as be,_ as ge}from"./index-CON1Pm6Z.js";import{S as _e,M as ye,P as xe}from"./MetricsBar-qlJWi86N.js";import{s as ke,r as Ce,N as Be}from"./RadioGroup-C7Qf7SCG.js";import{u as we,b as Re,c as Pe,N as Se,a as Le}from"./use-message-BmUESwx_.js";const $e=L("breadcrumb",`
 white-space: nowrap;
 cursor: default;
 line-height: var(--n-item-line-height);
`,[w("ul",`
 list-style: none;
 padding: 0;
 margin: 0;
 `),w("a",`
 color: inherit;
 text-decoration: inherit;
 `),L("breadcrumb-item",`
 font-size: var(--n-font-size);
 transition: color .3s var(--n-bezier);
 display: inline-flex;
 align-items: center;
 `,[L("icon",`
 font-size: 18px;
 vertical-align: -.2em;
 transition: color .3s var(--n-bezier);
 color: var(--n-item-text-color);
 `),w("&:not(:last-child)",[q("clickable",[E("link",`
 cursor: pointer;
 `,[w("&:hover",`
 background-color: var(--n-item-color-hover);
 `),w("&:active",`
 background-color: var(--n-item-color-pressed); 
 `)])])]),E("link",`
 padding: 4px;
 border-radius: var(--n-item-border-radius);
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier);
 color: var(--n-item-text-color);
 position: relative;
 `,[w("&:hover",`
 color: var(--n-item-text-color-hover);
 `,[L("icon",`
 color: var(--n-item-text-color-hover);
 `)]),w("&:active",`
 color: var(--n-item-text-color-pressed);
 `,[L("icon",`
 color: var(--n-item-text-color-pressed);
 `)])]),E("separator",`
 margin: 0 8px;
 color: var(--n-separator-color);
 transition: color .3s var(--n-bezier);
 user-select: none;
 -webkit-user-select: none;
 `),w("&:last-child",[E("link",`
 font-weight: var(--n-font-weight-active);
 cursor: unset;
 color: var(--n-item-text-color-active);
 `,[L("icon",`
 color: var(--n-item-text-color-active);
 `)]),E("separator",`
 display: none;
 `)])])]),Z=ee("n-breadcrumb"),Ne=Object.assign(Object.assign({},J.props),{separator:{type:String,default:"/"}}),ze=O({name:"Breadcrumb",props:Ne,setup(e){const{mergedClsPrefixRef:o,inlineThemeDisabled:l}=Q(e),u=J("Breadcrumb","-breadcrumb",$e,W,e,o);te(Z,{separatorRef:ae(e,"separator"),mergedClsPrefixRef:o});const s=P(()=>{const{common:{cubicBezierEaseInOut:p},self:{separatorColor:c,itemTextColor:r,itemTextColorHover:h,itemTextColorPressed:b,itemTextColorActive:x,fontSize:k,fontWeightActive:N,itemBorderRadius:z,itemColorHover:T,itemColorPressed:D,itemLineHeight:v}}=u.value;return{"--n-font-size":k,"--n-bezier":p,"--n-item-text-color":r,"--n-item-text-color-hover":h,"--n-item-text-color-pressed":b,"--n-item-text-color-active":x,"--n-separator-color":c,"--n-item-color-hover":T,"--n-item-color-pressed":D,"--n-item-border-radius":z,"--n-font-weight-active":N,"--n-item-line-height":v}}),i=l?X("breadcrumb",void 0,s,e):void 0;return{mergedClsPrefix:o,cssVars:l?void 0:s,themeClass:i==null?void 0:i.themeClass,onRender:i==null?void 0:i.onRender}},render(){var e;return(e=this.onRender)===null||e===void 0||e.call(this),B("nav",{class:[`${this.mergedClsPrefix}-breadcrumb`,this.themeClass],style:this.cssVars,"aria-label":"Breadcrumb"},B("ul",null,this.$slots))}});function Te(e=oe?window:null){const o=()=>{const{hash:s,host:i,hostname:p,href:c,origin:r,pathname:h,port:b,protocol:x,search:k}=(e==null?void 0:e.location)||{};return{hash:s,host:i,hostname:p,href:c,origin:r,pathname:h,port:b,protocol:x,search:k}},l=_(o()),u=()=>{l.value=o()};return Y(()=>{e&&(e.addEventListener("popstate",u),e.addEventListener("hashchange",u))}),se(()=>{e&&(e.removeEventListener("popstate",u),e.removeEventListener("hashchange",u))}),l}const De={separator:String,href:String,clickable:{type:Boolean,default:!0},showSeparator:{type:Boolean,default:!0},onClick:Function},K=O({name:"BreadcrumbItem",props:De,slots:Object,setup(e,{slots:o}){const l=ne(Z,null);if(!l)return()=>null;const{separatorRef:u,mergedClsPrefixRef:s}=l,i=Te(),p=P(()=>e.href?"a":"span"),c=P(()=>i.value.href===e.href?"location":null);return()=>{const{value:r}=s;return B("li",{class:[`${r}-breadcrumb-item`,e.clickable&&`${r}-breadcrumb-item--clickable`]},B(p.value,{class:`${r}-breadcrumb-item__link`,"aria-current":c.value,href:e.href,onClick:e.onClick},o),e.showSeparator&&B("span",{class:`${r}-breadcrumb-item__separator`,"aria-hidden":"true"},re(o.separator,()=>{var h;return[(h=e.separator)!==null&&h!==void 0?h:u.value]})))}}}),G=O({name:"RadioButton",props:Ce,setup:ke,render(){const{mergedClsPrefix:e}=this;return B("label",{class:[`${e}-radio-button`,this.mergedDisabled&&`${e}-radio-button--disabled`,this.renderSafeChecked&&`${e}-radio-button--checked`,this.focus&&[`${e}-radio-button--focus`]]},B("input",{ref:"inputRef",type:"radio",class:`${e}-radio-input`,value:this.value,name:this.mergedName,checked:this.renderSafeChecked,disabled:this.mergedDisabled,onChange:this.handleRadioInputChange,onFocus:this.handleRadioInputFocus,onBlur:this.handleRadioInputBlur}),B("div",{class:`${e}-radio-button__state-border`}),le(this.$slots.default,o=>!o&&!this.label?null:B("div",{ref:"labelRef",class:`${e}-radio__label`},o||this.label)))}});function Ee(e,o){return V("subscribe_log",{projectId:e,onEvent:o})}function He(e,o,l,u){return V("read_log_history",{projectId:e,date:o,offset:l,limit:u})}function je(e){return V("list_log_dates",{projectId:e})}function Ie(e,o){return V("clear_log",{projectId:e,date:null})}let H=e=>console.error("[log store]",e);const Me=256*1024,Oe=ie("log",()=>{const e=_("live"),o=_(""),l=_(!1),u=_([]),s=_(""),i=_(0),p=_(!1);let c=null,r=null;async function h(v){if(r===v&&c)return;b(),e.value="live",o.value="",l.value=!0;const f=new ce;f.onmessage=t=>{o.value+=t.text},c=f,r=v;const[,y]=await j(()=>Ee(v,f));l.value=!1,y&&(H(`订阅实时日志失败：${y}`),b())}function b(){c=null,r=null}async function x(v){const[f,y]=await j(()=>je(v));if(y){H(`读取日志日期失败：${y}`);return}u.value=f!=null?f:[]}async function k(v,f,y){var m;b(),e.value="history",y&&(s.value=f,i.value=0,o.value=""),l.value=!0;const[t,a]=await j(()=>He(v,f,i.value,Me));if(l.value=!1,a){H(`读取历史日志失败：${a}`);return}t&&(y?o.value=t.data:o.value+=t.data,i.value=(m=t.next_offset)!=null?m:i.value,p.value=t.next_offset!==null)}async function N(v){!p.value||!s.value||await k(v,s.value,!1)}async function z(v){const[,f]=await j(()=>Ie(v));return f?(H(`清空日志失败：${f}`),!1):(e.value==="live"&&(o.value=""),!0)}function T(){b(),e.value="live",o.value="",l.value=!1,u.value=[],s.value="",i.value=0,p.value=!1}function D(v){H=v}return{mode:e,lines:o,loading:l,dates:u,historyDate:s,historyHasMore:p,startLive:h,stopLive:b,fetchDates:x,loadHistory:k,loadNextPage:N,clear:z,reset:T,setNotifier:D}}),Ve={class:"detail-page"},Ae={class:"topbar"},Fe={class:"info-card"},Ue={class:"info-head"},Ke={class:"name"},Ge={class:"info-meta"},Je=["title"],Ye={class:"val code"},Ze=["title"],qe={class:"val code"},Qe={class:"meta-row"},Xe={class:"val"},We={key:0,class:"metrics-wrap"},et={class:"log-card"},tt={class:"log-header"},at={key:0,class:"history-controls"},st={class:"log-actions"},ot={class:"line-count"},rt={class:"log-body"},nt=O({__name:"ProjectDetail",setup(e){const o=pe(),l=he(),u=ue(),s=Oe(),i=we(),p=Re(),c=P(()=>Number(o.params.id)),r=_(null),h=_(""),b=_(null),x=P(()=>{var t;return(t=u.statuses[c.value])!=null?t:null}),k=P(()=>{var t,a;return(a=(t=x.value)==null?void 0:t.health)!=null?a:"stopped"});Y(async()=>{s.setNotifier(m=>i.error(m));const[t,a]=await u.safe(()=>de(c.value));if(a||!t){h.value=a!=null?a:"项目不存在";return}r.value=t,u.startPolling(),await s.startLive(c.value)}),ve(()=>{s.stopLive(),s.reset()}),fe(()=>s.lines,async()=>{if(s.mode!=="live")return;await be();const t=b.value;t&&(t.scrollTop=t.scrollHeight)});async function N(t){if(t!==s.mode)if(t==="live")await s.startLive(c.value);else{await s.fetchDates(c.value);const a=s.dates[0];a?await s.loadHistory(c.value,a,!0):s.lines=""}}const z=P(()=>s.dates.map(t=>({label:`${t.slice(0,4)}-${t.slice(4,6)}-${t.slice(6,8)}`,value:t})));async function T(t){t&&await s.loadHistory(c.value,t,!0)}async function D(){await s.loadNextPage(c.value)}async function v(){p.warning({title:"清空确认",content:"确定清空当天日志吗？此操作不可恢复。",positiveText:"清空",negativeText:"取消",onPositiveClick:async()=>{await s.clear(c.value)&&i.success("已清空当日日志")}})}function f(){l.back()}const y=P(()=>{const t=s.lines;if(!t)return 0;let a=1;for(let m=0;m<t.length;m++)t.charCodeAt(m)===10&&a++;return a});return(t,a)=>($(),I("div",Ve,[n("div",Ae,[g(d(ze),null,{default:C(()=>[g(d(K),{clickable:"",onClick:f},{default:C(()=>[...a[2]||(a[2]=[S("项目",-1)])]),_:1}),g(d(K),null,{default:C(()=>{var m,A;return[S(R((A=(m=r.value)==null?void 0:m.name)!=null?A:"..."),1)]}),_:1})]),_:1})]),h.value?($(),F(d(Pe),{key:0,description:h.value},null,8,["description"])):r.value?($(),I(me,{key:1},[n("div",Fe,[n("div",Ue,[n("span",Ke,R(r.value.name),1),g(d(Se),{size:"small",type:"primary",bordered:!1},{default:C(()=>[S(R(d(xe)[r.value.type]),1)]),_:1}),g(_e,{health:k.value},null,8,["health"])]),n("div",Ge,[n("div",{class:"meta-row",title:r.value.path},[a[3]||(a[3]=n("span",{class:"lbl"},"路径",-1)),n("span",Ye,R(r.value.path||"-"),1)],8,Je),n("div",{class:"meta-row",title:r.value.start_cmd},[a[4]||(a[4]=n("span",{class:"lbl"},"启动",-1)),n("span",qe,R(r.value.start_cmd||"-"),1)],8,Ze),n("div",Qe,[a[5]||(a[5]=n("span",{class:"lbl"},"端口",-1)),n("span",Xe,R(r.value.expected_ports.length?r.value.expected_ports.join(" / "):"-"),1)])]),x.value&&k.value!=="stopped"?($(),I("div",We,[g(ye,{status:x.value},null,8,["status"])])):M("",!0)]),n("div",et,[n("div",tt,[g(d(Be),{value:d(s).mode,size:"small","onUpdate:value":a[0]||(a[0]=m=>N(m))},{default:C(()=>[g(d(G),{value:"live"},{default:C(()=>[...a[6]||(a[6]=[S("实时",-1)])]),_:1}),g(d(G),{value:"history"},{default:C(()=>[...a[7]||(a[7]=[S("历史",-1)])]),_:1})]),_:1},8,["value"]),d(s).mode==="history"?($(),I("div",at,[g(d(Le),{value:d(s).historyDate,options:z.value,size:"small",placeholder:"选择日期",style:{width:"150px"},"onUpdate:value":a[1]||(a[1]=m=>T(m))},null,8,["value","options"]),g(d(U),{size:"small",disabled:!d(s).historyHasMore,onClick:D},{default:C(()=>[...a[8]||(a[8]=[S(" 加载更多 ",-1)])]),_:1},8,["disabled"])])):M("",!0),n("div",st,[n("span",ot,R(y.value)+" 行",1),d(s).mode==="live"?($(),F(d(U),{key:0,size:"small",tertiary:"",onClick:v},{default:C(()=>[...a[9]||(a[9]=[S(" 清空 ",-1)])]),_:1})):M("",!0)])]),n("div",rt,[n("pre",{ref_key:"logBox",ref:b,class:"log-text"},R(d(s).lines||"（暂无日志）"),513)])])],64)):M("",!0)]))}}),dt=ge(nt,[["__scopeId","data-v-2ef35946"]]);export{dt as default};
