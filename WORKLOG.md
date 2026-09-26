# WORKLOG

2026-09-26 · 调查项目是否有中文语言 · 只读，无代码改动 · 遍历 src/ 与全仓 UTF-8 文本，确认无 i18n/locale 文件、无中文字面量 · 结论：无中文本地化（UI 字符串硬编码英文）
2026-09-26 · 引入 i18n 并配置联合国六种官方语言 · 新增 src/ui/i18n.rs、locales/{en,zh-CN,fr,es,ru,ar}.json、temp/i18n/* 脚本；批量将 src/ui 中 1300+ 处英文字面量包裹为 crate::tr!()；sidebar 增加语言切换控件并在 app.rs 处理切换/持久化；main.rs 启动初始化 · 验证：cargo check 0 warning、cargo test 136 通过（含 4 个 i18n 单测）、cargo build 成功，启动截图确认界面已中文显示 · 待办：其余约 590 个长文案仍回退英文，可按需补全 locales（生成器 temp/i18n/gen_locales.py 会校验 key）
2026-09-26 · 语言选择改为下拉点选 · src/ui/components/sidebar.rs 新增 language_button()，用 gpui-component 的 DropdownMenu/PopupMenuItem 弹出六语言列表（带当前项勾选），点击即切换；移除 next_locale 循环逻辑 · 验证：cargo fmt/check/test 全通过，截图确认下拉菜单正确弹出
