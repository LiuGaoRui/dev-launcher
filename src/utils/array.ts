/** 原地移动数组中的元素：将 from 位置的元素移到 to 位置，其余顺延。 */
export function moveInArray<T>(arr: T[], from: number, to: number): boolean {
  if (from < 0 || to < 0 || from >= arr.length || to >= arr.length) return false
  arr.splice(to, 0, arr.splice(from, 1)[0])
  return true
}
