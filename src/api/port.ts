// port 命令薄包装。
// 对齐 src-tauri/src/commands/port.rs：list_listening_ports + 端口忽略 CRUD。
// 杀进程复用 cleaner 域的 kill_dev_processes（见 api/cleaner.ts）。

import { invokeCmd } from './invoke'
import type { ListeningPortInfo } from '@/types/port'

/**
 * 扫描全系统监听端口（TCP LISTEN，v4+v6），分类评分后按关注度降序返回。
 *
 * 后端拉取项目 expected_ports 做匹配标注，复用持久化 sysinfo System 刷新全进程；
 * 命中 port_ignore 忽略名单的端口统一返回 ignored 分类（不计入角标）。
 */
export function listListeningPorts(): Promise<ListeningPortInfo[]> {
  return invokeCmd<ListeningPortInfo[]>('list_listening_ports')
}

/**
 * 忽略一个端口（可疑行「忽略」按钮）。
 *
 * 按端口号持久化到后端 port_ignore 表（进程重启不影响），重复忽略幂等。
 * 忽略后该端口不再提示、不计入角标，可在「已忽略」折叠区恢复。
 */
export function addPortIgnore(port: number): Promise<void> {
  return invokeCmd<void>('add_port_ignore', { port })
}

/** 恢复一个端口（「已忽略」折叠区「恢复」按钮） */
export function removePortIgnore(port: number): Promise<void> {
  return invokeCmd<void>('remove_port_ignore', { port })
}
