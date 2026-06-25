import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './style.css'
import './styles/theme.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

// 注：Naive UI 的 n-config-provider / message / dialog / loadingBar
// 不在此处 app.use，而是在 App.vue 根组件模板内用组件包裹，
// 以便 useMessage()/useDialog() 等组合式 API 能正确取到 provider 上下文。

app.mount('#app')
