import{l as p,n as b,p as z,m as S,f as B,j as f,x as J,y as U,H as ie,t as $,aY as je,k as Y,P as H,O as Ee,e as P,D as Z,am as Oe,aZ as ae,v as Ne,i as T,u as Ie,a_ as re,a$ as He,aU as Te,b0 as Me,_ as se,F as Q,b1 as Fe,b2 as De,b3 as Ke,X as le,c as ce,b4 as Ve,b5 as qe,S as Ue,E as Xe,ab as N,N as Ge,b6 as Je,G as Ye,L as Ze,M as Qe,Z as We,b7 as et,b8 as tt,b9 as ot,ba as nt,$ as it,ad as at,J as de,bb as rt,bc as st,bd as lt,aJ as k,be as ct,bf as dt,af as ut,bg as ft,bh as vt,bi as ht,aT as X,aE as pt,aF as mt,aX as F,aV as gt,bj as bt,a as xt,w as yt,aC as D,aD as C,aM as A,aK as j,aI as ee,aP as E,aQ as K,aa as te,aB as I,aL as O,aW as wt,bk as Ct,aN as kt}from"./index-BM8qy5Px.js";import{s as _t,r as zt,z as ue,y as Pt,q as Lt,w as Rt,S as St,M as At,k as $t,g as Bt,P as jt}from"./MetricsBar-CQWqrmU6.js";const Et=p("breadcrumb",`
 white-space: nowrap;
 cursor: default;
 line-height: var(--n-item-line-height);
`,[b("ul",`
 list-style: none;
 padding: 0;
 margin: 0;
 `),b("a",`
 color: inherit;
 text-decoration: inherit;
 `),p("breadcrumb-item",`
 font-size: var(--n-font-size);
 transition: color .3s var(--n-bezier);
 display: inline-flex;
 align-items: center;
 `,[p("icon",`
 font-size: 18px;
 vertical-align: -.2em;
 transition: color .3s var(--n-bezier);
 color: var(--n-item-text-color);
 `),b("&:not(:last-child)",[z("clickable",[S("link",`
 cursor: pointer;
 `,[b("&:hover",`
 background-color: var(--n-item-color-hover);
 `),b("&:active",`
 background-color: var(--n-item-color-pressed); 
 `)])])]),S("link",`
 padding: 4px;
 border-radius: var(--n-item-border-radius);
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier);
 color: var(--n-item-text-color);
 position: relative;
 `,[b("&:hover",`
 color: var(--n-item-text-color-hover);
 `,[p("icon",`
 color: var(--n-item-text-color-hover);
 `)]),b("&:active",`
 color: var(--n-item-text-color-pressed);
 `,[p("icon",`
 color: var(--n-item-text-color-pressed);
 `)])]),S("separator",`
 margin: 0 8px;
 color: var(--n-separator-color);
 transition: color .3s var(--n-bezier);
 user-select: none;
 -webkit-user-select: none;
 `),b("&:last-child",[S("link",`
 font-weight: var(--n-font-weight-active);
 cursor: unset;
 color: var(--n-item-text-color-active);
 `,[p("icon",`
 color: var(--n-item-text-color-active);
 `)]),S("separator",`
 display: none;
 `)])])]),fe=Y("n-breadcrumb"),Ot=Object.assign(Object.assign({},U.props),{separator:{type:String,default:"/"}}),Nt=B({name:"Breadcrumb",props:Ot,setup(e){const{mergedClsPrefixRef:t,inlineThemeDisabled:n}=J(e),i=U("Breadcrumb","-breadcrumb",Et,je,e,t);H(fe,{separatorRef:Ee(e,"separator"),mergedClsPrefixRef:t});const o=$(()=>{const{common:{cubicBezierEaseInOut:l},self:{separatorColor:r,itemTextColor:c,itemTextColorHover:h,itemTextColorPressed:v,itemTextColorActive:g,fontSize:_,fontWeightActive:w,itemBorderRadius:a,itemColorHover:y,itemColorPressed:m,itemLineHeight:x}}=i.value;return{"--n-font-size":_,"--n-bezier":l,"--n-item-text-color":c,"--n-item-text-color-hover":h,"--n-item-text-color-pressed":v,"--n-item-text-color-active":g,"--n-separator-color":r,"--n-item-color-hover":y,"--n-item-color-pressed":m,"--n-item-border-radius":a,"--n-font-weight-active":w,"--n-item-line-height":x}}),s=n?ie("breadcrumb",void 0,o,e):void 0;return{mergedClsPrefix:t,cssVars:n?void 0:o,themeClass:s==null?void 0:s.themeClass,onRender:s==null?void 0:s.onRender}},render(){var e;return(e=this.onRender)===null||e===void 0||e.call(this),f("nav",{class:[`${this.mergedClsPrefix}-breadcrumb`,this.themeClass],style:this.cssVars,"aria-label":"Breadcrumb"},f("ul",null,this.$slots))}});function It(e=ae?window:null){const t=()=>{const{hash:o,host:s,hostname:l,href:r,origin:c,pathname:h,port:v,protocol:g,search:_}=(e==null?void 0:e.location)||{};return{hash:o,host:s,hostname:l,href:r,origin:c,pathname:h,port:v,protocol:g,search:_}},n=P(t()),i=()=>{n.value=t()};return Z(()=>{e&&(e.addEventListener("popstate",i),e.addEventListener("hashchange",i))}),Oe(()=>{e&&(e.removeEventListener("popstate",i),e.removeEventListener("hashchange",i))}),n}const Ht={separator:String,href:String,clickable:{type:Boolean,default:!0},showSeparator:{type:Boolean,default:!0},onClick:Function},oe=B({name:"BreadcrumbItem",props:Ht,slots:Object,setup(e,{slots:t}){const n=T(fe,null);if(!n)return()=>null;const{separatorRef:i,mergedClsPrefixRef:o}=n,s=It(),l=$(()=>e.href?"a":"span"),r=$(()=>s.value.href===e.href?"location":null);return()=>{const{value:c}=o;return f("li",{class:[`${c}-breadcrumb-item`,e.clickable&&`${c}-breadcrumb-item--clickable`]},f(l.value,{class:`${c}-breadcrumb-item__link`,"aria-current":r.value,href:e.href,onClick:e.onClick},t),e.showSeparator&&f("span",{class:`${c}-breadcrumb-item__separator`,"aria-hidden":"true"},Ne(t.separator,()=>{var h;return[(h=e.separator)!==null&&h!==void 0?h:i.value]})))}}}),ne=B({name:"RadioButton",props:zt,setup:_t,render(){const{mergedClsPrefix:e}=this;return f("label",{class:[`${e}-radio-button`,this.mergedDisabled&&`${e}-radio-button--disabled`,this.renderSafeChecked&&`${e}-radio-button--checked`,this.focus&&[`${e}-radio-button--focus`]]},f("input",{ref:"inputRef",type:"radio",class:`${e}-radio-input`,value:this.value,name:this.mergedName,checked:this.renderSafeChecked,disabled:this.mergedDisabled,onChange:this.handleRadioInputChange,onFocus:this.handleRadioInputFocus,onBlur:this.handleRadioInputBlur}),f("div",{class:`${e}-radio-button__state-border`}),Ie(this.$slots.default,t=>!t&&!this.label?null:f("div",{ref:"labelRef",class:`${e}-radio__label`},t||this.label)))}});function Tt(){const e=T(He,null);return e===null&&re("use-loading-bar","No outer <n-loading-bar-provider /> founded."),e}const Mt=B({name:"ModalEnvironment",props:Object.assign(Object.assign({},Me),{internalKey:{type:String,required:!0},onInternalAfterLeave:{type:Function,required:!0}}),setup(e){const t=P(!0);function n(){const{onInternalAfterLeave:v,internalKey:g,onAfterLeave:_}=e;v&&v(g),_&&_()}function i(){const{onPositiveClick:v}=e;v?Promise.resolve(v()).then(g=>{g!==!1&&c()}):c()}function o(){const{onNegativeClick:v}=e;v?Promise.resolve(v()).then(g=>{g!==!1&&c()}):c()}function s(){const{onClose:v}=e;v?Promise.resolve(v()).then(g=>{g!==!1&&c()}):c()}function l(v){const{onMaskClick:g,maskClosable:_}=e;g&&(g(v),_&&c())}function r(){const{onEsc:v}=e;v&&v()}function c(){t.value=!1}function h(v){t.value=v}return{show:t,hide:c,handleUpdateShow:h,handleAfterLeave:n,handleCloseClick:s,handleNegativeClick:o,handlePositiveClick:i,handleMaskClick:l,handleEsc:r}},render(){const{handleUpdateShow:e,handleAfterLeave:t,handleMaskClick:n,handleEsc:i,show:o}=this;return f(Te,Object.assign({},this.$props,{show:o,onUpdateShow:e,onMaskClick:n,onEsc:i,onAfterLeave:t,internalAppear:!0,internalModal:!0}),this.$slots)}}),Ft={to:[String,Object]},Dt=B({name:"ModalProvider",props:Ft,setup(){const e=P([]),t={};function n(l={}){const r=le(),c=ce(Object.assign(Object.assign({},l),{key:r,destroy:()=>{var h;(h=t[`n-modal-${r}`])===null||h===void 0||h.hide()}}));return e.value.push(c),c}function i(l){const{value:r}=e;r.splice(r.findIndex(c=>c.key===l),1)}function o(){Object.values(t).forEach(l=>{l==null||l.hide()})}const s={create:n,destroyAll:o};return H(Ve,s),H(Ke,{clickedRef:De(64),clickedPositionRef:Fe()}),H(qe,e),Object.assign(Object.assign({},s),{modalList:e,modalInstRefs:t,handleAfterLeave:i})},render(){var e,t;return f(Q,null,[this.modalList.map(n=>{var i;return f(Mt,se(n,["destroy","render"],{to:(i=n.to)!==null&&i!==void 0?i:this.to,ref:o=>{o===null?delete this.modalInstRefs[`n-modal-${n.key}`]:this.modalInstRefs[`n-modal-${n.key}`]=o},internalKey:n.key,onInternalAfterLeave:this.handleAfterLeave}),{default:n.render})}),(t=(e=this.$slots).default)===null||t===void 0?void 0:t.call(e)])}}),G=Y("n-notification-provider"),Kt=B({name:"NotificationContainer",props:{scrollable:{type:Boolean,required:!0},placement:{type:String,required:!0}},setup(){const{mergedThemeRef:e,mergedClsPrefixRef:t,wipTransitionCountRef:n}=T(G),i=P(null);return Xe(()=>{var o,s;n.value>0?(o=i==null?void 0:i.value)===null||o===void 0||o.classList.add("transitioning"):(s=i==null?void 0:i.value)===null||s===void 0||s.classList.remove("transitioning")}),{selfRef:i,mergedTheme:e,mergedClsPrefix:t,transitioning:n}},render(){const{$slots:e,scrollable:t,mergedClsPrefix:n,mergedTheme:i,placement:o}=this;return f("div",{ref:"selfRef",class:[`${n}-notification-container`,t&&`${n}-notification-container--scrollable`,`${n}-notification-container--${o}`]},t?f(Ue,{theme:i.peers.Scrollbar,themeOverrides:i.peerOverrides.Scrollbar,contentStyle:{overflow:"hidden"}},e):e)}}),Vt={info:()=>f(nt,null),success:()=>f(ot,null),warning:()=>f(tt,null),error:()=>f(et,null),default:()=>null},W={closable:{type:Boolean,default:!0},type:{type:String,default:"default"},avatar:Function,title:[String,Function],description:[String,Function],content:[String,Function],meta:[String,Function],action:[String,Function],onClose:{type:Function,required:!0},keepAliveOnHover:Boolean,onMouseenter:Function,onMouseleave:Function},qt=We(W),Ut=B({name:"Notification",props:W,setup(e){const{mergedClsPrefixRef:t,mergedThemeRef:n,props:i}=T(G),{inlineThemeDisabled:o,mergedRtlRef:s}=J(),l=Ye("Notification",s,t),r=$(()=>{const{type:h}=e,{self:{color:v,textColor:g,closeIconColor:_,closeIconColorHover:w,closeIconColorPressed:a,headerTextColor:y,descriptionTextColor:m,actionTextColor:x,borderRadius:L,headerFontWeight:u,boxShadow:d,lineHeight:R,fontSize:M,closeMargin:he,closeSize:pe,width:me,padding:ge,closeIconSize:be,closeBorderRadius:xe,closeColorHover:ye,closeColorPressed:we,titleFontSize:Ce,metaFontSize:ke,descriptionFontSize:_e,[Ze("iconColor",h)]:ze},common:{cubicBezierEaseOut:Pe,cubicBezierEaseIn:Le,cubicBezierEaseInOut:Re}}=n.value,{left:Se,right:Ae,top:$e,bottom:Be}=Qe(ge);return{"--n-color":v,"--n-font-size":M,"--n-text-color":g,"--n-description-text-color":m,"--n-action-text-color":x,"--n-title-text-color":y,"--n-title-font-weight":u,"--n-bezier":Re,"--n-bezier-ease-out":Pe,"--n-bezier-ease-in":Le,"--n-border-radius":L,"--n-box-shadow":d,"--n-close-border-radius":xe,"--n-close-color-hover":ye,"--n-close-color-pressed":we,"--n-close-icon-color":_,"--n-close-icon-color-hover":w,"--n-close-icon-color-pressed":a,"--n-line-height":R,"--n-icon-color":ze,"--n-close-margin":he,"--n-close-size":pe,"--n-close-icon-size":be,"--n-width":me,"--n-padding-left":Se,"--n-padding-right":Ae,"--n-padding-top":$e,"--n-padding-bottom":Be,"--n-title-font-size":Ce,"--n-meta-font-size":ke,"--n-description-font-size":_e}}),c=o?ie("notification",$(()=>e.type[0]),r,i):void 0;return{mergedClsPrefix:t,showAvatar:$(()=>e.avatar||e.type!=="default"),handleCloseClick(){e.onClose()},rtlEnabled:l,cssVars:o?void 0:r,themeClass:c==null?void 0:c.themeClass,onRender:c==null?void 0:c.onRender}},render(){var e;const{mergedClsPrefix:t}=this;return(e=this.onRender)===null||e===void 0||e.call(this),f("div",{class:[`${t}-notification-wrapper`,this.themeClass],onMouseenter:this.onMouseenter,onMouseleave:this.onMouseleave,style:this.cssVars},f("div",{class:[`${t}-notification`,this.rtlEnabled&&`${t}-notification--rtl`,this.themeClass,{[`${t}-notification--closable`]:this.closable,[`${t}-notification--show-avatar`]:this.showAvatar}],style:this.cssVars},this.showAvatar?f("div",{class:`${t}-notification__avatar`},this.avatar?N(this.avatar):this.type!=="default"?f(Ge,{clsPrefix:t},{default:()=>Vt[this.type]()}):null):null,this.closable?f(Je,{clsPrefix:t,class:`${t}-notification__close`,onClick:this.handleCloseClick}):null,f("div",{ref:"bodyRef",class:`${t}-notification-main`},this.title?f("div",{class:`${t}-notification-main__header`},N(this.title)):null,this.description?f("div",{class:`${t}-notification-main__description`},N(this.description)):null,this.content?f("pre",{class:`${t}-notification-main__content`},N(this.content)):null,this.meta||this.action?f("div",{class:`${t}-notification-main-footer`},this.meta?f("div",{class:`${t}-notification-main-footer__meta`},N(this.meta)):null,this.action?f("div",{class:`${t}-notification-main-footer__action`},N(this.action)):null):null)))}}),Xt=Object.assign(Object.assign({},W),{duration:Number,onClose:Function,onLeave:Function,onAfterEnter:Function,onAfterLeave:Function,onHide:Function,onAfterShow:Function,onAfterHide:Function}),Gt=B({name:"NotificationEnvironment",props:Object.assign(Object.assign({},Xt),{internalKey:{type:String,required:!0},onInternalAfterLeave:{type:Function,required:!0}}),setup(e){const{wipTransitionCountRef:t}=T(G),n=P(!0);let i=null;function o(){n.value=!1,i&&window.clearTimeout(i)}function s(a){t.value++,de(()=>{a.style.height=`${a.offsetHeight}px`,a.style.maxHeight="0",a.style.transition="none",a.offsetHeight,a.style.transition="",a.style.maxHeight=a.style.height})}function l(a){t.value--,a.style.height="",a.style.maxHeight="";const{onAfterEnter:y,onAfterShow:m}=e;y&&y(),m&&m()}function r(a){t.value++,a.style.maxHeight=`${a.offsetHeight}px`,a.style.height=`${a.offsetHeight}px`,a.offsetHeight}function c(a){const{onHide:y}=e;y&&y(),a.style.maxHeight="0",a.offsetHeight}function h(){t.value--;const{onAfterLeave:a,onInternalAfterLeave:y,onAfterHide:m,internalKey:x}=e;a&&a(),y(x),m&&m()}function v(){const{duration:a}=e;a&&(i=window.setTimeout(o,a))}function g(a){a.currentTarget===a.target&&i!==null&&(window.clearTimeout(i),i=null)}function _(a){a.currentTarget===a.target&&v()}function w(){const{onClose:a}=e;a?Promise.resolve(a()).then(y=>{y!==!1&&o()}):o()}return Z(()=>{e.duration&&(i=window.setTimeout(o,e.duration))}),{show:n,hide:o,handleClose:w,handleAfterLeave:h,handleLeave:c,handleBeforeLeave:r,handleAfterEnter:l,handleBeforeEnter:s,handleMouseenter:g,handleMouseleave:_}},render(){return f(at,{name:"notification-transition",appear:!0,onBeforeEnter:this.handleBeforeEnter,onAfterEnter:this.handleAfterEnter,onBeforeLeave:this.handleBeforeLeave,onLeave:this.handleLeave,onAfterLeave:this.handleAfterLeave},{default:()=>this.show?f(Ut,Object.assign({},it(this.$props,qt),{onClose:this.handleClose,onMouseenter:this.duration&&this.keepAliveOnHover?this.handleMouseenter:void 0,onMouseleave:this.duration&&this.keepAliveOnHover?this.handleMouseleave:void 0})):null})}}),Jt=b([p("notification-container",`
 z-index: 4000;
 position: fixed;
 overflow: visible;
 display: flex;
 flex-direction: column;
 align-items: flex-end;
 `,[b(">",[p("scrollbar",`
 width: initial;
 overflow: visible;
 height: -moz-fit-content !important;
 height: fit-content !important;
 max-height: 100vh !important;
 `,[b(">",[p("scrollbar-container",`
 height: -moz-fit-content !important;
 height: fit-content !important;
 max-height: 100vh !important;
 `,[p("scrollbar-content",`
 padding-top: 12px;
 padding-bottom: 33px;
 `)])])])]),z("top, top-right, top-left",`
 top: 12px;
 `,[b("&.transitioning >",[p("scrollbar",[b(">",[p("scrollbar-container",`
 min-height: 100vh !important;
 `)])])])]),z("bottom, bottom-right, bottom-left",`
 bottom: 12px;
 `,[b(">",[p("scrollbar",[b(">",[p("scrollbar-container",[p("scrollbar-content",`
 padding-bottom: 12px;
 `)])])])]),p("notification-wrapper",`
 display: flex;
 align-items: flex-end;
 margin-bottom: 0;
 margin-top: 12px;
 `)]),z("top, bottom",`
 left: 50%;
 transform: translateX(-50%);
 `,[p("notification-wrapper",[b("&.notification-transition-enter-from, &.notification-transition-leave-to",`
 transform: scale(0.85);
 `),b("&.notification-transition-leave-from, &.notification-transition-enter-to",`
 transform: scale(1);
 `)])]),z("top",[p("notification-wrapper",`
 transform-origin: top center;
 `)]),z("bottom",[p("notification-wrapper",`
 transform-origin: bottom center;
 `)]),z("top-right, bottom-right",[p("notification",`
 margin-left: 28px;
 margin-right: 16px;
 `)]),z("top-left, bottom-left",[p("notification",`
 margin-left: 16px;
 margin-right: 28px;
 `)]),z("top-right",`
 right: 0;
 `,[V("top-right")]),z("top-left",`
 left: 0;
 `,[V("top-left")]),z("bottom-right",`
 right: 0;
 `,[V("bottom-right")]),z("bottom-left",`
 left: 0;
 `,[V("bottom-left")]),z("scrollable",[z("top-right",`
 top: 0;
 `),z("top-left",`
 top: 0;
 `),z("bottom-right",`
 bottom: 0;
 `),z("bottom-left",`
 bottom: 0;
 `)]),p("notification-wrapper",`
 margin-bottom: 12px;
 `,[b("&.notification-transition-enter-from, &.notification-transition-leave-to",`
 opacity: 0;
 margin-top: 0 !important;
 margin-bottom: 0 !important;
 `),b("&.notification-transition-leave-from, &.notification-transition-enter-to",`
 opacity: 1;
 `),b("&.notification-transition-leave-active",`
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier),
 opacity .3s var(--n-bezier),
 transform .3s var(--n-bezier-ease-in),
 max-height .3s var(--n-bezier),
 margin-top .3s linear,
 margin-bottom .3s linear,
 box-shadow .3s var(--n-bezier);
 `),b("&.notification-transition-enter-active",`
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier),
 opacity .3s var(--n-bezier),
 transform .3s var(--n-bezier-ease-out),
 max-height .3s var(--n-bezier),
 margin-top .3s linear,
 margin-bottom .3s linear,
 box-shadow .3s var(--n-bezier);
 `)]),p("notification",`
 background-color: var(--n-color);
 color: var(--n-text-color);
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier),
 opacity .3s var(--n-bezier),
 box-shadow .3s var(--n-bezier);
 font-family: inherit;
 font-size: var(--n-font-size);
 font-weight: 400;
 position: relative;
 display: flex;
 overflow: hidden;
 flex-shrink: 0;
 padding-left: var(--n-padding-left);
 padding-right: var(--n-padding-right);
 width: var(--n-width);
 max-width: calc(100vw - 16px - 16px);
 border-radius: var(--n-border-radius);
 box-shadow: var(--n-box-shadow);
 box-sizing: border-box;
 opacity: 1;
 `,[S("avatar",[p("icon",`
 color: var(--n-icon-color);
 `),p("base-icon",`
 color: var(--n-icon-color);
 `)]),z("show-avatar",[p("notification-main",`
 margin-left: 40px;
 width: calc(100% - 40px); 
 `)]),z("closable",[p("notification-main",[b("> *:first-child",`
 padding-right: 20px;
 `)]),S("close",`
 position: absolute;
 top: 0;
 right: 0;
 margin: var(--n-close-margin);
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier);
 `)]),S("avatar",`
 position: absolute;
 top: var(--n-padding-top);
 left: var(--n-padding-left);
 width: 28px;
 height: 28px;
 font-size: 28px;
 display: flex;
 align-items: center;
 justify-content: center;
 `,[p("icon","transition: color .3s var(--n-bezier);")]),p("notification-main",`
 padding-top: var(--n-padding-top);
 padding-bottom: var(--n-padding-bottom);
 box-sizing: border-box;
 display: flex;
 flex-direction: column;
 margin-left: 8px;
 width: calc(100% - 8px);
 `,[p("notification-main-footer",`
 display: flex;
 align-items: center;
 justify-content: space-between;
 margin-top: 12px;
 `,[S("meta",`
 font-size: var(--n-meta-font-size);
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-description-text-color);
 `),S("action",`
 cursor: pointer;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-action-text-color);
 `)]),S("header",`
 font-weight: var(--n-title-font-weight);
 font-size: var(--n-title-font-size);
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-title-text-color);
 `),S("description",`
 margin-top: 8px;
 font-size: var(--n-description-font-size);
 white-space: pre-wrap;
 word-wrap: break-word;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-description-text-color);
 `),S("content",`
 line-height: var(--n-line-height);
 margin: 12px 0 0 0;
 font-family: inherit;
 white-space: pre-wrap;
 word-wrap: break-word;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-text-color);
 `,[b("&:first-child","margin: 0;")])])])])]);function V(e){const n=e.split("-")[1]==="left"?"calc(-100%)":"calc(100%)";return p("notification-wrapper",[b("&.notification-transition-enter-from, &.notification-transition-leave-to",`
 transform: translate(${n}, 0);
 `),b("&.notification-transition-leave-from, &.notification-transition-enter-to",`
 transform: translate(0, 0);
 `)])}const ve=Y("n-notification-api"),Yt=Object.assign(Object.assign({},U.props),{containerClass:String,containerStyle:[String,Object],to:[String,Object],scrollable:{type:Boolean,default:!0},max:Number,placement:{type:String,default:"top-right"},keepAliveOnHover:Boolean}),Zt=B({name:"NotificationProvider",props:Yt,setup(e){const{mergedClsPrefixRef:t}=J(e),n=P([]),i={},o=new Set;function s(w){const a=le(),y=()=>{o.add(a),i[a]&&i[a].hide()},m=ce(Object.assign(Object.assign({},w),{key:a,destroy:y,hide:y,deactivate:y})),{max:x}=e;if(x&&n.value.length-o.size>=x){let L=!1,u=0;for(const d of n.value){if(!o.has(d.key)){i[d.key]&&(d.destroy(),L=!0);break}u++}L||n.value.splice(u,1)}return n.value.push(m),m}const l=["info","success","warning","error"].map(w=>a=>s(Object.assign(Object.assign({},a),{type:w})));function r(w){o.delete(w),n.value.splice(n.value.findIndex(a=>a.key===w),1)}const c=U("Notification","-notification",Jt,st,e,t),h={create:s,info:l[0],success:l[1],warning:l[2],error:l[3],open:g,destroyAll:_},v=P(0);H(ve,h),H(G,{props:e,mergedClsPrefixRef:t,mergedThemeRef:c,wipTransitionCountRef:v});function g(w){return s(w)}function _(){Object.values(n.value).forEach(w=>{w.hide()})}return Object.assign({mergedClsPrefix:t,notificationList:n,notificationRefs:i,handleAfterLeave:r},h)},render(){var e,t,n;const{placement:i}=this;return f(Q,null,(t=(e=this.$slots).default)===null||t===void 0?void 0:t.call(e),this.notificationList.length?f(rt,{to:(n=this.to)!==null&&n!==void 0?n:"body"},f(Kt,{class:this.containerClass,style:this.containerStyle,scrollable:this.scrollable&&i!=="top"&&i!=="bottom",placement:i},{default:()=>this.notificationList.map(o=>f(Gt,Object.assign({ref:s=>{const l=o.key;s===null?delete this.notificationRefs[l]:this.notificationRefs[l]=s}},se(o,["destroy","hide","deactivate"]),{internalKey:o.key,onInternalAfterLeave:this.handleAfterLeave,keepAliveOnHover:o.keepAliveOnHover===void 0?this.keepAliveOnHover:o.keepAliveOnHover})))})):null)}});function Qt(){const e=T(ve,null);return e===null&&re("use-notification","No outer `n-notification-provider` found."),e}const Wt=B({name:"InjectionExtractor",props:{onSetup:Function},setup(e,{slots:t}){var n;return(n=e.onSetup)===null||n===void 0||n.call(e),()=>{var i;return(i=t.default)===null||i===void 0?void 0:i.call(t)}}}),eo={message:Pt,notification:Qt,loadingBar:Tt,dialog:ue,modal:ct};function to({providersAndProps:e,configProviderProps:t}){let n=lt(o);const i={app:n};function o(){return f(dt,k(t),{default:()=>e.map(({type:r,Provider:c,props:h})=>f(c,k(h),{default:()=>f(Wt,{onSetup:()=>i[r]=eo[r]()})}))})}let s;return ae&&(s=document.createElement("div"),document.body.appendChild(s),n.mount(s)),Object.assign({unmount:()=>{var r;if(n===null||s===null){ut("discrete","unmount call no need because discrete app has been unmounted");return}n.unmount(),(r=s.parentNode)===null||r===void 0||r.removeChild(s),s=null,n=null}},i)}function oo(e,{configProviderProps:t,messageProviderProps:n,dialogProviderProps:i,notificationProviderProps:o,loadingBarProviderProps:s,modalProviderProps:l}={}){const r=[];return e.forEach(h=>{switch(h){case"message":r.push({type:h,Provider:ht,props:n});break;case"notification":r.push({type:h,Provider:Zt,props:o});break;case"dialog":r.push({type:h,Provider:vt,props:i});break;case"loadingBar":r.push({type:h,Provider:ft,props:s});break;case"modal":r.push({type:h,Provider:Dt,props:l})}}),to({providersAndProps:r,configProviderProps:t})}function no(e,t){return X("subscribe_log",{projectId:e,onEvent:t})}function io(e,t,n,i){return X("read_log_history",{projectId:e,date:t,offset:n,limit:i})}function ao(e){return X("list_log_dates",{projectId:e})}function ro(e,t){return X("clear_log",{projectId:e,date:null})}const{message:q}=oo(["message"]),so=256*1024,lo=pt("log",()=>{const e=P("live"),t=P(""),n=P(!1),i=P([]),o=P(""),s=P(0),l=P(!1);let r=null,c=null;async function h(m){if(c===m&&r)return;v(),e.value="live",t.value="",n.value=!0;const x=new mt;x.onmessage=u=>{t.value+=u.text},r=x,c=m;const[,L]=await F(()=>no(m,x));n.value=!1,L&&(q.error(`订阅实时日志失败：${L}`),v())}function v(){r=null,c=null}async function g(m){const[x,L]=await F(()=>ao(m));if(L){q.error(`读取日志日期失败：${L}`);return}i.value=x!=null?x:[]}async function _(m,x,L){var R;v(),e.value="history",L&&(o.value=x,s.value=0,t.value=""),n.value=!0;const[u,d]=await F(()=>io(m,x,s.value,so));if(n.value=!1,d){q.error(`读取历史日志失败：${d}`);return}u&&(L?t.value=u.data:t.value+=u.data,s.value=(R=u.next_offset)!=null?R:s.value,l.value=u.next_offset!==null)}async function w(m){!l.value||!o.value||await _(m,o.value,!1)}async function a(m){const[,x]=await F(()=>ro(m));return x?(q.error(`清空日志失败：${x}`),!1):(e.value==="live"&&(t.value=""),!0)}function y(){v(),e.value="live",t.value="",n.value=!1,i.value=[],o.value="",s.value=0,l.value=!1}return{mode:e,lines:t,loading:n,dates:i,historyDate:o,historyHasMore:l,startLive:h,stopLive:v,fetchDates:g,loadHistory:_,loadNextPage:w,clear:a,reset:y}}),co={class:"detail-page"},uo={class:"topbar"},fo={class:"info-card card-surface"},vo={class:"info-head"},ho={class:"name"},po={class:"info-meta"},mo=["title"],go={class:"val code"},bo=["title"],xo={class:"val code"},yo={class:"meta-row"},wo={class:"val"},Co={key:0,class:"metrics-wrap"},ko={class:"log-card card-surface"},_o={class:"log-header"},zo={key:0,class:"history-controls"},Po={class:"log-actions"},Lo={class:"line-count"},Ro={class:"log-body"},So=B({__name:"ProjectDetail",setup(e){const t=Ct(),n=wt(),i=gt(),o=lo(),s=ue(),l=$(()=>Number(t.params.id)),r=P(null),c=P(""),h=P(null),v=$(()=>{var u;return(u=i.statuses[l.value])!=null?u:null}),g=$(()=>{var u,d;return(d=(u=v.value)==null?void 0:u.health)!=null?d:"stopped"});Z(async()=>{const[u,d]=await i.safe(()=>bt(l.value));if(d||!u){c.value=d!=null?d:"项目不存在";return}r.value=u,i.startPolling(),await o.startLive(l.value)}),xt(()=>{o.stopLive(),o.reset()}),yt(()=>o.lines,async()=>{if(o.mode!=="live")return;await de();const u=h.value;u&&(u.scrollTop=u.scrollHeight)});async function _(u){if(u!==o.mode)if(u==="live")await o.startLive(l.value);else{await o.fetchDates(l.value);const d=o.dates[0];d?await o.loadHistory(l.value,d,!0):o.lines=""}}const w=$(()=>o.dates.map(u=>({label:`${u.slice(0,4)}-${u.slice(4,6)}-${u.slice(6,8)}`,value:u})));async function a(u){u&&await o.loadHistory(l.value,u,!0)}async function y(){await o.loadNextPage(l.value)}async function m(){s.warning({title:"清空确认",content:"确定清空当天日志吗？此操作不可恢复。",positiveText:"清空",negativeText:"取消",onPositiveClick:async()=>{await o.clear(l.value)}})}function x(){n.back()}const L=$(()=>{const u=o.lines;if(!u)return 0;let d=1;for(let R=0;R<u.length;R++)u.charCodeAt(R)===10&&d++;return d});return(u,d)=>(I(),D("div",co,[C("div",uo,[A(k(Nt),null,{default:j(()=>[A(k(oe),{clickable:"",onClick:x},{default:j(()=>[...d[2]||(d[2]=[O("项目",-1)])]),_:1}),A(k(oe),null,{default:j(()=>{var R,M;return[O(E((M=(R=r.value)==null?void 0:R.name)!=null?M:"..."),1)]}),_:1})]),_:1})]),c.value?(I(),ee(k(Lt),{key:0,description:c.value},null,8,["description"])):r.value?(I(),D(Q,{key:1},[C("div",fo,[C("div",vo,[C("span",ho,E(r.value.name),1),A(k(Rt),{size:"small",type:"primary",bordered:!1},{default:j(()=>[O(E(k(jt)[r.value.type]),1)]),_:1}),A(St,{health:g.value},null,8,["health"])]),C("div",po,[C("div",{class:"meta-row",title:r.value.path},[d[3]||(d[3]=C("span",{class:"lbl"},"路径",-1)),C("span",go,E(r.value.path||"-"),1)],8,mo),C("div",{class:"meta-row",title:r.value.start_cmd},[d[4]||(d[4]=C("span",{class:"lbl"},"启动",-1)),C("span",xo,E(r.value.start_cmd||"-"),1)],8,bo),C("div",yo,[d[5]||(d[5]=C("span",{class:"lbl"},"端口",-1)),C("span",wo,E(r.value.expected_ports.length?r.value.expected_ports.join(" / "):"-"),1)])]),v.value&&g.value!=="stopped"?(I(),D("div",Co,[A(At,{status:v.value},null,8,["status"])])):K("",!0)]),C("div",ko,[C("div",_o,[A(k($t),{value:k(o).mode,size:"small","onUpdate:value":d[0]||(d[0]=R=>_(R))},{default:j(()=>[A(k(ne),{value:"live"},{default:j(()=>[...d[6]||(d[6]=[O("实时",-1)])]),_:1}),A(k(ne),{value:"history"},{default:j(()=>[...d[7]||(d[7]=[O("历史",-1)])]),_:1})]),_:1},8,["value"]),k(o).mode==="history"?(I(),D("div",zo,[A(k(Bt),{value:k(o).historyDate,options:w.value,size:"small",placeholder:"选择日期",style:{width:"150px"},"onUpdate:value":d[1]||(d[1]=R=>a(R))},null,8,["value","options"]),A(k(te),{size:"small",disabled:!k(o).historyHasMore,onClick:y},{default:j(()=>[...d[8]||(d[8]=[O(" 加载更多 ",-1)])]),_:1},8,["disabled"])])):K("",!0),C("div",Po,[C("span",Lo,E(L.value)+" 行",1),k(o).mode==="live"?(I(),ee(k(te),{key:0,size:"small",tertiary:"",onClick:m},{default:j(()=>[...d[9]||(d[9]=[O(" 清空 ",-1)])]),_:1})):K("",!0)])]),C("div",Ro,[C("pre",{ref_key:"logBox",ref:h,class:"log-text"},E(k(o).lines||"（暂无日志）"),513)])])],64)):K("",!0)]))}}),Bo=kt(So,[["__scopeId","data-v-8b934c85"]]);export{Bo as default};
