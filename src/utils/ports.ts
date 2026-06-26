// 端口字符串解析 —— 扫描 / 手动添加表单共用的端口输入处理。
//
// 用户在「预期端口」输入框里用逗号或空白分隔多个端口，提交前需拆成数组并剔除空串。

/**
 * 把逗号/空白（含中文逗号）分隔的端口字符串拆成数组，剔除空串。
 */
export function parsePorts(input: string): string[] {
  return input
    .split(/[,，\s]+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
}
