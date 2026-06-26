import{l as p,n as b,p as z,m as R,f as $,j as d,x as G,y as q,H as ne,t as A,aX as je,k as J,P as H,O as Ee,e as P,D as Y,am as Oe,aY as ie,v as Ne,i as T,u as Ie,aZ as ae,a_ as He,aU as Te,a$ as Me,_ as re,F as Z,b0 as Fe,b1 as De,b2 as Ke,X as se,c as le,b3 as Ve,b4 as qe,S as Ue,E as Xe,ab as N,N as Ge,b5 as Je,G as Ye,L as Ze,M as Qe,Z as We,b6 as et,b7 as tt,b8 as ot,b9 as nt,$ as it,ad as at,J as ce,ba as rt,bb as st,bc as lt,aJ as k,bd as ct,be as dt,af as ut,bf as ft,bg as vt,bh as ht,aT as U,aE as pt,aF as mt,bi as M,aV as gt,bj as bt,a as xt,w as yt,aC as F,aD as C,aM as S,aK as j,aI as W,aP as E,aQ as D,aa as ee,aB as I,aL as O,aW as wt,bk as Ct,aN as kt}from"./index-BOQVE-AK.js";import{s as _t,r as zt,z as de,y as Pt,q as Lt,w as Rt,S as St,M as At,k as $t,g as Bt,P as jt}from"./MetricsBar-C2aoHdyw.js";const Et=p("breadcrumb",`
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
 `),b("&:not(:last-child)",[z("clickable",[R("link",`
 cursor: pointer;
 `,[b("&:hover",`
 background-color: var(--n-item-color-hover);
 `),b("&:active",`
 background-color: var(--n-item-color-pressed); 
 `)])])]),R("link",`
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
 `)])]),R("separator",`
 margin: 0 8px;
 color: var(--n-separator-color);
 transition: color .3s var(--n-bezier);
 user-select: none;
 -webkit-user-select: none;
 `),b("&:last-child",[R("link",`
 font-weight: var(--n-font-weight-active);
 cursor: unset;
 color: var(--n-item-text-color-active);
 `,[p("icon",`
 color: var(--n-item-text-color-active);
 `)]),R("separator",`
 display: none;
 `)])])]),ue=J("n-breadcrumb"),Ot=Object.assign(Object.assign({},q.props),{separator:{type:String,default:"/"}}),Nt=$({name:"Breadcrumb",props:Ot,setup(e){const{mergedClsPrefixRef:t,inlineThemeDisabled:n}=G(e),i=q("Breadcrumb","-breadcrumb",Et,je,e,t);H(ue,{separatorRef:Ee(e,"separator"),mergedClsPrefixRef:t});const o=A(()=>{const{common:{cubicBezierEaseInOut:l},self:{separatorColor:r,itemTextColor:c,itemTextColorHover:h,itemTextColorPressed:u,itemTextColorActive:g,fontSize:_,fontWeightActive:w,itemBorderRadius:a,itemColorHover:y,itemColorPressed:m,itemLineHeight:x}}=i.value;return{"--n-font-size":_,"--n-bezier":l,"--n-item-text-color":c,"--n-item-text-color-hover":h,"--n-item-text-color-pressed":u,"--n-item-text-color-active":g,"--n-separator-color":r,"--n-item-color-hover":y,"--n-item-color-pressed":m,"--n-item-border-radius":a,"--n-font-weight-active":w,"--n-item-line-height":x}}),s=n?ne("breadcrumb",void 0,o,e):void 0;return{mergedClsPrefix:t,cssVars:n?void 0:o,themeClass:s?.themeClass,onRender:s?.onRender}},render(){var e;return(e=this.onRender)===null||e===void 0||e.call(this),d("nav",{class:[`${this.mergedClsPrefix}-breadcrumb`,this.themeClass],style:this.cssVars,"aria-label":"Breadcrumb"},d("ul",null,this.$slots))}});function It(e=ie?window:null){const t=()=>{const{hash:o,host:s,hostname:l,href:r,origin:c,pathname:h,port:u,protocol:g,search:_}=e?.location||{};return{hash:o,host:s,hostname:l,href:r,origin:c,pathname:h,port:u,protocol:g,search:_}},n=P(t()),i=()=>{n.value=t()};return Y(()=>{e&&(e.addEventListener("popstate",i),e.addEventListener("hashchange",i))}),Oe(()=>{e&&(e.removeEventListener("popstate",i),e.removeEventListener("hashchange",i))}),n}const Ht={separator:String,href:String,clickable:{type:Boolean,default:!0},showSeparator:{type:Boolean,default:!0},onClick:Function},te=$({name:"BreadcrumbItem",props:Ht,slots:Object,setup(e,{slots:t}){const n=T(ue,null);if(!n)return()=>null;const{separatorRef:i,mergedClsPrefixRef:o}=n,s=It(),l=A(()=>e.href?"a":"span"),r=A(()=>s.value.href===e.href?"location":null);return()=>{const{value:c}=o;return d("li",{class:[`${c}-breadcrumb-item`,e.clickable&&`${c}-breadcrumb-item--clickable`]},d(l.value,{class:`${c}-breadcrumb-item__link`,"aria-current":r.value,href:e.href,onClick:e.onClick},t),e.showSeparator&&d("span",{class:`${c}-breadcrumb-item__separator`,"aria-hidden":"true"},Ne(t.separator,()=>{var h;return[(h=e.separator)!==null&&h!==void 0?h:i.value]})))}}}),oe=$({name:"RadioButton",props:zt,setup:_t,render(){const{mergedClsPrefix:e}=this;return d("label",{class:[`${e}-radio-button`,this.mergedDisabled&&`${e}-radio-button--disabled`,this.renderSafeChecked&&`${e}-radio-button--checked`,this.focus&&[`${e}-radio-button--focus`]]},d("input",{ref:"inputRef",type:"radio",class:`${e}-radio-input`,value:this.value,name:this.mergedName,checked:this.renderSafeChecked,disabled:this.mergedDisabled,onChange:this.handleRadioInputChange,onFocus:this.handleRadioInputFocus,onBlur:this.handleRadioInputBlur}),d("div",{class:`${e}-radio-button__state-border`}),Ie(this.$slots.default,t=>!t&&!this.label?null:d("div",{ref:"labelRef",class:`${e}-radio__label`},t||this.label)))}});function Tt(){const e=T(He,null);return e===null&&ae("use-loading-bar","No outer <n-loading-bar-provider /> founded."),e}const Mt=$({name:"ModalEnvironment",props:Object.assign(Object.assign({},Me),{internalKey:{type:String,required:!0},onInternalAfterLeave:{type:Function,required:!0}}),setup(e){const t=P(!0);function n(){const{onInternalAfterLeave:u,internalKey:g,onAfterLeave:_}=e;u&&u(g),_&&_()}function i(){const{onPositiveClick:u}=e;u?Promise.resolve(u()).then(g=>{g!==!1&&c()}):c()}function o(){const{onNegativeClick:u}=e;u?Promise.resolve(u()).then(g=>{g!==!1&&c()}):c()}function s(){const{onClose:u}=e;u?Promise.resolve(u()).then(g=>{g!==!1&&c()}):c()}function l(u){const{onMaskClick:g,maskClosable:_}=e;g&&(g(u),_&&c())}function r(){const{onEsc:u}=e;u&&u()}function c(){t.value=!1}function h(u){t.value=u}return{show:t,hide:c,handleUpdateShow:h,handleAfterLeave:n,handleCloseClick:s,handleNegativeClick:o,handlePositiveClick:i,handleMaskClick:l,handleEsc:r}},render(){const{handleUpdateShow:e,handleAfterLeave:t,handleMaskClick:n,handleEsc:i,show:o}=this;return d(Te,Object.assign({},this.$props,{show:o,onUpdateShow:e,onMaskClick:n,onEsc:i,onAfterLeave:t,internalAppear:!0,internalModal:!0}),this.$slots)}}),Ft={to:[String,Object]},Dt=$({name:"ModalProvider",props:Ft,setup(){const e=P([]),t={};function n(l={}){const r=se(),c=le(Object.assign(Object.assign({},l),{key:r,destroy:()=>{var h;(h=t[`n-modal-${r}`])===null||h===void 0||h.hide()}}));return e.value.push(c),c}function i(l){const{value:r}=e;r.splice(r.findIndex(c=>c.key===l),1)}function o(){Object.values(t).forEach(l=>{l?.hide()})}const s={create:n,destroyAll:o};return H(Ve,s),H(Ke,{clickedRef:De(64),clickedPositionRef:Fe()}),H(qe,e),Object.assign(Object.assign({},s),{modalList:e,modalInstRefs:t,handleAfterLeave:i})},render(){var e,t;return d(Z,null,[this.modalList.map(n=>{var i;return d(Mt,re(n,["destroy","render"],{to:(i=n.to)!==null&&i!==void 0?i:this.to,ref:o=>{o===null?delete this.modalInstRefs[`n-modal-${n.key}`]:this.modalInstRefs[`n-modal-${n.key}`]=o},internalKey:n.key,onInternalAfterLeave:this.handleAfterLeave}),{default:n.render})}),(t=(e=this.$slots).default)===null||t===void 0?void 0:t.call(e)])}}),X=J("n-notification-provider"),Kt=$({name:"NotificationContainer",props:{scrollable:{type:Boolean,required:!0},placement:{type:String,required:!0}},setup(){const{mergedThemeRef:e,mergedClsPrefixRef:t,wipTransitionCountRef:n}=T(X),i=P(null);return Xe(()=>{var o,s;n.value>0?(o=i?.value)===null||o===void 0||o.classList.add("transitioning"):(s=i?.value)===null||s===void 0||s.classList.remove("transitioning")}),{selfRef:i,mergedTheme:e,mergedClsPrefix:t,transitioning:n}},render(){const{$slots:e,scrollable:t,mergedClsPrefix:n,mergedTheme:i,placement:o}=this;return d("div",{ref:"selfRef",class:[`${n}-notification-container`,t&&`${n}-notification-container--scrollable`,`${n}-notification-container--${o}`]},t?d(Ue,{theme:i.peers.Scrollbar,themeOverrides:i.peerOverrides.Scrollbar,contentStyle:{overflow:"hidden"}},e):e)}}),Vt={info:()=>d(nt,null),success:()=>d(ot,null),warning:()=>d(tt,null),error:()=>d(et,null),default:()=>null},Q={closable:{type:Boolean,default:!0},type:{type:String,default:"default"},avatar:Function,title:[String,Function],description:[String,Function],content:[String,Function],meta:[String,Function],action:[String,Function],onClose:{type:Function,required:!0},keepAliveOnHover:Boolean,onMouseenter:Function,onMouseleave:Function},qt=We(Q),Ut=$({name:"Notification",props:Q,setup(e){const{mergedClsPrefixRef:t,mergedThemeRef:n,props:i}=T(X),{inlineThemeDisabled:o,mergedRtlRef:s}=G(),l=Ye("Notification",s,t),r=A(()=>{const{type:h}=e,{self:{color:u,textColor:g,closeIconColor:_,closeIconColorHover:w,closeIconColorPressed:a,headerTextColor:y,descriptionTextColor:m,actionTextColor:x,borderRadius:L,headerFontWeight:v,boxShadow:f,lineHeight:B,fontSize:ve,closeMargin:he,closeSize:pe,width:me,padding:ge,closeIconSize:be,closeBorderRadius:xe,closeColorHover:ye,closeColorPressed:we,titleFontSize:Ce,metaFontSize:ke,descriptionFontSize:_e,[Ze("iconColor",h)]:ze},common:{cubicBezierEaseOut:Pe,cubicBezierEaseIn:Le,cubicBezierEaseInOut:Re}}=n.value,{left:Se,right:Ae,top:$e,bottom:Be}=Qe(ge);return{"--n-color":u,"--n-font-size":ve,"--n-text-color":g,"--n-description-text-color":m,"--n-action-text-color":x,"--n-title-text-color":y,"--n-title-font-weight":v,"--n-bezier":Re,"--n-bezier-ease-out":Pe,"--n-bezier-ease-in":Le,"--n-border-radius":L,"--n-box-shadow":f,"--n-close-border-radius":xe,"--n-close-color-hover":ye,"--n-close-color-pressed":we,"--n-close-icon-color":_,"--n-close-icon-color-hover":w,"--n-close-icon-color-pressed":a,"--n-line-height":B,"--n-icon-color":ze,"--n-close-margin":he,"--n-close-size":pe,"--n-close-icon-size":be,"--n-width":me,"--n-padding-left":Se,"--n-padding-right":Ae,"--n-padding-top":$e,"--n-padding-bottom":Be,"--n-title-font-size":Ce,"--n-meta-font-size":ke,"--n-description-font-size":_e}}),c=o?ne("notification",A(()=>e.type[0]),r,i):void 0;return{mergedClsPrefix:t,showAvatar:A(()=>e.avatar||e.type!=="default"),handleCloseClick(){e.onClose()},rtlEnabled:l,cssVars:o?void 0:r,themeClass:c?.themeClass,onRender:c?.onRender}},render(){var e;const{mergedClsPrefix:t}=this;return(e=this.onRender)===null||e===void 0||e.call(this),d("div",{class:[`${t}-notification-wrapper`,this.themeClass],onMouseenter:this.onMouseenter,onMouseleave:this.onMouseleave,style:this.cssVars},d("div",{class:[`${t}-notification`,this.rtlEnabled&&`${t}-notification--rtl`,this.themeClass,{[`${t}-notification--closable`]:this.closable,[`${t}-notification--show-avatar`]:this.showAvatar}],style:this.cssVars},this.showAvatar?d("div",{class:`${t}-notification__avatar`},this.avatar?N(this.avatar):this.type!=="default"?d(Ge,{clsPrefix:t},{default:()=>Vt[this.type]()}):null):null,this.closable?d(Je,{clsPrefix:t,class:`${t}-notification__close`,onClick:this.handleCloseClick}):null,d("div",{ref:"bodyRef",class:`${t}-notification-main`},this.title?d("div",{class:`${t}-notification-main__header`},N(this.title)):null,this.description?d("div",{class:`${t}-notification-main__description`},N(this.description)):null,this.content?d("pre",{class:`${t}-notification-main__content`},N(this.content)):null,this.meta||this.action?d("div",{class:`${t}-notification-main-footer`},this.meta?d("div",{class:`${t}-notification-main-footer__meta`},N(this.meta)):null,this.action?d("div",{class:`${t}-notification-main-footer__action`},N(this.action)):null):null)))}}),Xt=Object.assign(Object.assign({},Q),{duration:Number,onClose:Function,onLeave:Function,onAfterEnter:Function,onAfterLeave:Function,onHide:Function,onAfterShow:Function,onAfterHide:Function}),Gt=$({name:"NotificationEnvironment",props:Object.assign(Object.assign({},Xt),{internalKey:{type:String,required:!0},onInternalAfterLeave:{type:Function,required:!0}}),setup(e){const{wipTransitionCountRef:t}=T(X),n=P(!0);let i=null;function o(){n.value=!1,i&&window.clearTimeout(i)}function s(a){t.value++,ce(()=>{a.style.height=`${a.offsetHeight}px`,a.style.maxHeight="0",a.style.transition="none",a.offsetHeight,a.style.transition="",a.style.maxHeight=a.style.height})}function l(a){t.value--,a.style.height="",a.style.maxHeight="";const{onAfterEnter:y,onAfterShow:m}=e;y&&y(),m&&m()}function r(a){t.value++,a.style.maxHeight=`${a.offsetHeight}px`,a.style.height=`${a.offsetHeight}px`,a.offsetHeight}function c(a){const{onHide:y}=e;y&&y(),a.style.maxHeight="0",a.offsetHeight}function h(){t.value--;const{onAfterLeave:a,onInternalAfterLeave:y,onAfterHide:m,internalKey:x}=e;a&&a(),y(x),m&&m()}function u(){const{duration:a}=e;a&&(i=window.setTimeout(o,a))}function g(a){a.currentTarget===a.target&&i!==null&&(window.clearTimeout(i),i=null)}function _(a){a.currentTarget===a.target&&u()}function w(){const{onClose:a}=e;a?Promise.resolve(a()).then(y=>{y!==!1&&o()}):o()}return Y(()=>{e.duration&&(i=window.setTimeout(o,e.duration))}),{show:n,hide:o,handleClose:w,handleAfterLeave:h,handleLeave:c,handleBeforeLeave:r,handleAfterEnter:l,handleBeforeEnter:s,handleMouseenter:g,handleMouseleave:_}},render(){return d(at,{name:"notification-transition",appear:!0,onBeforeEnter:this.handleBeforeEnter,onAfterEnter:this.handleAfterEnter,onBeforeLeave:this.handleBeforeLeave,onLeave:this.handleLeave,onAfterLeave:this.handleAfterLeave},{default:()=>this.show?d(Ut,Object.assign({},it(this.$props,qt),{onClose:this.handleClose,onMouseenter:this.duration&&this.keepAliveOnHover?this.handleMouseenter:void 0,onMouseleave:this.duration&&this.keepAliveOnHover?this.handleMouseleave:void 0})):null})}}),Jt=b([p("notification-container",`
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
 `,[K("top-right")]),z("top-left",`
 left: 0;
 `,[K("top-left")]),z("bottom-right",`
 right: 0;
 `,[K("bottom-right")]),z("bottom-left",`
 left: 0;
 `,[K("bottom-left")]),z("scrollable",[z("top-right",`
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
 `,[R("avatar",[p("icon",`
 color: var(--n-icon-color);
 `),p("base-icon",`
 color: var(--n-icon-color);
 `)]),z("show-avatar",[p("notification-main",`
 margin-left: 40px;
 width: calc(100% - 40px); 
 `)]),z("closable",[p("notification-main",[b("> *:first-child",`
 padding-right: 20px;
 `)]),R("close",`
 position: absolute;
 top: 0;
 right: 0;
 margin: var(--n-close-margin);
 transition:
 background-color .3s var(--n-bezier),
 color .3s var(--n-bezier);
 `)]),R("avatar",`
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
 `,[R("meta",`
 font-size: var(--n-meta-font-size);
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-description-text-color);
 `),R("action",`
 cursor: pointer;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-action-text-color);
 `)]),R("header",`
 font-weight: var(--n-title-font-weight);
 font-size: var(--n-title-font-size);
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-title-text-color);
 `),R("description",`
 margin-top: 8px;
 font-size: var(--n-description-font-size);
 white-space: pre-wrap;
 word-wrap: break-word;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-description-text-color);
 `),R("content",`
 line-height: var(--n-line-height);
 margin: 12px 0 0 0;
 font-family: inherit;
 white-space: pre-wrap;
 word-wrap: break-word;
 transition: color .3s var(--n-bezier-ease-out);
 color: var(--n-text-color);
 `,[b("&:first-child","margin: 0;")])])])])]);function K(e){const n=e.split("-")[1]==="left"?"calc(-100%)":"calc(100%)";return p("notification-wrapper",[b("&.notification-transition-enter-from, &.notification-transition-leave-to",`
 transform: translate(${n}, 0);
 `),b("&.notification-transition-leave-from, &.notification-transition-enter-to",`
 transform: translate(0, 0);
 `)])}const fe=J("n-notification-api"),Yt=Object.assign(Object.assign({},q.props),{containerClass:String,containerStyle:[String,Object],to:[String,Object],scrollable:{type:Boolean,default:!0},max:Number,placement:{type:String,default:"top-right"},keepAliveOnHover:Boolean}),Zt=$({name:"NotificationProvider",props:Yt,setup(e){const{mergedClsPrefixRef:t}=G(e),n=P([]),i={},o=new Set;function s(w){const a=se(),y=()=>{o.add(a),i[a]&&i[a].hide()},m=le(Object.assign(Object.assign({},w),{key:a,destroy:y,hide:y,deactivate:y})),{max:x}=e;if(x&&n.value.length-o.size>=x){let L=!1,v=0;for(const f of n.value){if(!o.has(f.key)){i[f.key]&&(f.destroy(),L=!0);break}v++}L||n.value.splice(v,1)}return n.value.push(m),m}const l=["info","success","warning","error"].map(w=>a=>s(Object.assign(Object.assign({},a),{type:w})));function r(w){o.delete(w),n.value.splice(n.value.findIndex(a=>a.key===w),1)}const c=q("Notification","-notification",Jt,st,e,t),h={create:s,info:l[0],success:l[1],warning:l[2],error:l[3],open:g,destroyAll:_},u=P(0);H(fe,h),H(X,{props:e,mergedClsPrefixRef:t,mergedThemeRef:c,wipTransitionCountRef:u});function g(w){return s(w)}function _(){Object.values(n.value).forEach(w=>{w.hide()})}return Object.assign({mergedClsPrefix:t,notificationList:n,notificationRefs:i,handleAfterLeave:r},h)},render(){var e,t,n;const{placement:i}=this;return d(Z,null,(t=(e=this.$slots).default)===null||t===void 0?void 0:t.call(e),this.notificationList.length?d(rt,{to:(n=this.to)!==null&&n!==void 0?n:"body"},d(Kt,{class:this.containerClass,style:this.containerStyle,scrollable:this.scrollable&&i!=="top"&&i!=="bottom",placement:i},{default:()=>this.notificationList.map(o=>d(Gt,Object.assign({ref:s=>{const l=o.key;s===null?delete this.notificationRefs[l]:this.notificationRefs[l]=s}},re(o,["destroy","hide","deactivate"]),{internalKey:o.key,onInternalAfterLeave:this.handleAfterLeave,keepAliveOnHover:o.keepAliveOnHover===void 0?this.keepAliveOnHover:o.keepAliveOnHover})))})):null)}});function Qt(){const e=T(fe,null);return e===null&&ae("use-notification","No outer `n-notification-provider` found."),e}const Wt=$({name:"InjectionExtractor",props:{onSetup:Function},setup(e,{slots:t}){var n;return(n=e.onSetup)===null||n===void 0||n.call(e),()=>{var i;return(i=t.default)===null||i===void 0?void 0:i.call(t)}}}),eo={message:Pt,notification:Qt,loadingBar:Tt,dialog:de,modal:ct};function to({providersAndProps:e,configProviderProps:t}){let n=lt(o);const i={app:n};function o(){return d(dt,k(t),{default:()=>e.map(({type:r,Provider:c,props:h})=>d(c,k(h),{default:()=>d(Wt,{onSetup:()=>i[r]=eo[r]()})}))})}let s;return ie&&(s=document.createElement("div"),document.body.appendChild(s),n.mount(s)),Object.assign({unmount:()=>{var r;if(n===null||s===null){ut("discrete","unmount call no need because discrete app has been unmounted");return}n.unmount(),(r=s.parentNode)===null||r===void 0||r.removeChild(s),s=null,n=null}},i)}function oo(e,{configProviderProps:t,messageProviderProps:n,dialogProviderProps:i,notificationProviderProps:o,loadingBarProviderProps:s,modalProviderProps:l}={}){const r=[];return e.forEach(h=>{switch(h){case"message":r.push({type:h,Provider:ht,props:n});break;case"notification":r.push({type:h,Provider:Zt,props:o});break;case"dialog":r.push({type:h,Provider:vt,props:i});break;case"loadingBar":r.push({type:h,Provider:ft,props:s});break;case"modal":r.push({type:h,Provider:Dt,props:l})}}),to({providersAndProps:r,configProviderProps:t})}function no(e,t){return U("subscribe_log",{projectId:e,onEvent:t})}function io(e,t,n,i){return U("read_log_history",{projectId:e,date:t,offset:n,limit:i})}function ao(e){return U("list_log_dates",{projectId:e})}function ro(e,t){return U("clear_log",{projectId:e,date:null})}const{message:V}=oo(["message"]),so=256*1024,lo=pt("log",()=>{const e=P("live"),t=P(""),n=P(!1),i=P([]),o=P(""),s=P(0),l=P(!1);let r=null,c=null;async function h(m){if(c===m&&r)return;u(),e.value="live",t.value="",n.value=!0;const x=new mt;x.onmessage=v=>{t.value+=v.text},r=x,c=m;const[,L]=await M(()=>no(m,x));n.value=!1,L&&(V.error(`订阅实时日志失败：${L}`),u())}function u(){r=null,c=null}async function g(m){const[x,L]=await M(()=>ao(m));if(L){V.error(`读取日志日期失败：${L}`);return}i.value=x??[]}async function _(m,x,L){u(),e.value="history",L&&(o.value=x,s.value=0,t.value=""),n.value=!0;const[v,f]=await M(()=>io(m,x,s.value,so));if(n.value=!1,f){V.error(`读取历史日志失败：${f}`);return}v&&(L?t.value=v.data:t.value+=v.data,s.value=v.next_offset??s.value,l.value=v.next_offset!==null)}async function w(m){!l.value||!o.value||await _(m,o.value,!1)}async function a(m){const[,x]=await M(()=>ro(m));return x?(V.error(`清空日志失败：${x}`),!1):(e.value==="live"&&(t.value=""),!0)}function y(){u(),e.value="live",t.value="",n.value=!1,i.value=[],o.value="",s.value=0,l.value=!1}return{mode:e,lines:t,loading:n,dates:i,historyDate:o,historyHasMore:l,startLive:h,stopLive:u,fetchDates:g,loadHistory:_,loadNextPage:w,clear:a,reset:y}}),co={class:"detail-page"},uo={class:"topbar"},fo={class:"info-card card-surface"},vo={class:"info-head"},ho={class:"name"},po={class:"info-meta"},mo=["title"],go={class:"val code"},bo=["title"],xo={class:"val code"},yo={class:"meta-row"},wo={class:"val"},Co={key:0,class:"metrics-wrap"},ko={class:"log-card card-surface"},_o={class:"log-header"},zo={key:0,class:"history-controls"},Po={class:"log-actions"},Lo={class:"line-count"},Ro={class:"log-body"},So=$({__name:"ProjectDetail",setup(e){const t=Ct(),n=wt(),i=gt(),o=lo(),s=de(),l=A(()=>Number(t.params.id)),r=P(null),c=P(""),h=P(null),u=A(()=>i.statuses[l.value]??null),g=A(()=>u.value?.health??"stopped");Y(async()=>{const[v,f]=await i.safe(()=>bt(l.value));if(f||!v){c.value=f??"项目不存在";return}r.value=v,i.startPolling(),await o.startLive(l.value)}),xt(()=>{o.stopLive(),o.reset()}),yt(()=>o.lines,async()=>{if(o.mode!=="live")return;await ce();const v=h.value;v&&(v.scrollTop=v.scrollHeight)});async function _(v){if(v!==o.mode)if(v==="live")await o.startLive(l.value);else{await o.fetchDates(l.value);const f=o.dates[0];f?await o.loadHistory(l.value,f,!0):o.lines=""}}const w=A(()=>o.dates.map(v=>({label:`${v.slice(0,4)}-${v.slice(4,6)}-${v.slice(6,8)}`,value:v})));async function a(v){v&&await o.loadHistory(l.value,v,!0)}async function y(){await o.loadNextPage(l.value)}async function m(){s.warning({title:"清空确认",content:"确定清空当天日志吗？此操作不可恢复。",positiveText:"清空",negativeText:"取消",onPositiveClick:async()=>{await o.clear(l.value)}})}function x(){n.back()}const L=A(()=>{const v=o.lines;if(!v)return 0;let f=1;for(let B=0;B<v.length;B++)v.charCodeAt(B)===10&&f++;return f});return(v,f)=>(I(),F("div",co,[C("div",uo,[S(k(Nt),null,{default:j(()=>[S(k(te),{clickable:"",onClick:x},{default:j(()=>[...f[2]||(f[2]=[O("项目",-1)])]),_:1}),S(k(te),null,{default:j(()=>[O(E(r.value?.name??"..."),1)]),_:1})]),_:1})]),c.value?(I(),W(k(Lt),{key:0,description:c.value},null,8,["description"])):r.value?(I(),F(Z,{key:1},[C("div",fo,[C("div",vo,[C("span",ho,E(r.value.name),1),S(k(Rt),{size:"small",type:"primary",bordered:!1},{default:j(()=>[O(E(k(jt)[r.value.type]),1)]),_:1}),S(St,{health:g.value},null,8,["health"])]),C("div",po,[C("div",{class:"meta-row",title:r.value.path},[f[3]||(f[3]=C("span",{class:"lbl"},"路径",-1)),C("span",go,E(r.value.path||"-"),1)],8,mo),C("div",{class:"meta-row",title:r.value.start_cmd},[f[4]||(f[4]=C("span",{class:"lbl"},"启动",-1)),C("span",xo,E(r.value.start_cmd||"-"),1)],8,bo),C("div",yo,[f[5]||(f[5]=C("span",{class:"lbl"},"端口",-1)),C("span",wo,E(r.value.expected_ports.length?r.value.expected_ports.join(" / "):"-"),1)])]),u.value&&g.value!=="stopped"?(I(),F("div",Co,[S(At,{status:u.value},null,8,["status"])])):D("",!0)]),C("div",ko,[C("div",_o,[S(k($t),{value:k(o).mode,size:"small","onUpdate:value":f[0]||(f[0]=B=>_(B))},{default:j(()=>[S(k(oe),{value:"live"},{default:j(()=>[...f[6]||(f[6]=[O("实时",-1)])]),_:1}),S(k(oe),{value:"history"},{default:j(()=>[...f[7]||(f[7]=[O("历史",-1)])]),_:1})]),_:1},8,["value"]),k(o).mode==="history"?(I(),F("div",zo,[S(k(Bt),{value:k(o).historyDate,options:w.value,size:"small",placeholder:"选择日期",style:{width:"150px"},"onUpdate:value":f[1]||(f[1]=B=>a(B))},null,8,["value","options"]),S(k(ee),{size:"small",disabled:!k(o).historyHasMore,onClick:y},{default:j(()=>[...f[8]||(f[8]=[O(" 加载更多 ",-1)])]),_:1},8,["disabled"])])):D("",!0),C("div",Po,[C("span",Lo,E(L.value)+" 行",1),k(o).mode==="live"?(I(),W(k(ee),{key:0,size:"small",tertiary:"",onClick:m},{default:j(()=>[...f[9]||(f[9]=[O(" 清空 ",-1)])]),_:1})):D("",!0)])]),C("div",Ro,[C("pre",{ref_key:"logBox",ref:h,class:"log-text"},E(k(o).lines||"（暂无日志）"),513)])])],64)):D("",!0)]))}}),Bo=kt(So,[["__scopeId","data-v-8b934c85"]]);export{Bo as default};
