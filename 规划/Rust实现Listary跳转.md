可以，而且**用 Rust 完全可以做到接近 Listary 的 `Ctrl+G` 效果**。核心并不是什么 Windows 私有黑科技，而是把问题拆成两个部分：

1. **持续知道“当前/最近一个 Explorer 窗口所在的目录”**
2. **在文件选择对话框里捕获 `Ctrl+G`，然后让这个对话框跳到那个目录**

Listary 官方对这个功能的描述就是：在文件选择窗口中，通过 `Ctrl+G` 快速跳转到目标文件夹；它还支持“点击已经打开的文件夹，再点击文件选择窗口后立即跳转”。([Listary][1])

---

# 一、先说结论：Rust 可以做到

我会把你的 Rust 实现设计成：

```text
┌──────────────────────────────────────────┐
│              Rust 后台进程                │
│                                          │
│  Explorer Tracker                        │
│       │                                  │
│       ├── 监听 Explorer 窗口              │
│       ├── 获取当前目录                   │
│       └── 保存最近活动目录               │
│                                          │
│  Keyboard Hook                           │
│       │                                  │
│       └── 捕获 Ctrl+G                    │
│                    │                     │
│                    ▼                     │
│             当前前台窗口                  │
│                    │                     │
│                    ▼                     │
│             判断是不是文件对话框           │
│                    │                     │
│                    ▼                     │
│            Explorer 最近目录              │
│                    │                     │
│                    ▼                     │
│             导航 File Dialog              │
└──────────────────────────────────────────┘
```

其中最关键的其实不是键盘 Hook，而是：

> **怎么从 Explorer 获取当前目录，以及怎么让任意应用的文件选择框切换目录。**

这两个地方决定了实现质量。

---

# 二、第一步：获取 Explorer 当前目录

这个其实比较优雅。

Windows Explorer 本身提供 Shell COM 接口，可以枚举 Explorer 窗口，然后获得当前路径。

例如概念上：

```text
ShellWindows
    │
    ├── Explorer Window #1
    │       └── C:\Users\xxx\Downloads
    │
    ├── Explorer Window #2
    │       └── D:\Projects\rust
    │
    └── Explorer Window #3
            └── E:\Photos
```

你可以持续维护：

```rust
struct ExplorerWindow {
    hwnd: HWND,
    path: PathBuf,
    last_active: Instant,
}
```

然后：

```text
HWND 0x1234 -> C:\Users\xxx\Downloads
HWND 0x5678 -> D:\Projects\rust
HWND 0x9ABC -> E:\Photos
```

最后根据：

```text
GetForegroundWindow()
```

判断哪个 Explorer 当前处于活动状态。

---

# 三、为什么不建议“读取 Explorer 地址栏文本”

你可能第一反应是：

```text
找到 Explorer
↓
找到地址栏 Edit
↓
读取文本
```

这个方法可以做，但我不推荐作为主要实现。

因为 Explorer 的 UI 在 Windows 不同版本、不同状态下变化比较多。

更合理的是：

```text
Explorer
   ↓
Shell COM
   ↓
IShellBrowser / IWebBrowserApp / ShellWindows
   ↓
当前 LocationURL / ShellItem
   ↓
Path
```

这也是比较符合 Windows Shell 架构的方式。

---

# 四、第二步：Ctrl+G 怎么捕获？

这里有几个选择。

## 方案 A：RegisterHotKey

例如：

```text
Ctrl + G
```

注册一个全局热键。

但这里有个问题：

> `RegisterHotKey` 更适合“全局快捷键”，不太适合你这种需要判断当前应用/当前控件上下文的场景。

因为你的需求实际上是：

```text
Ctrl+G
    ↓
当前是不是 File Dialog？
    ↓
是 → Listary 行为
不是 → 不干扰原程序
```

所以我更倾向于：

## 方案 B：WH_KEYBOARD_LL

Windows 提供：

```cpp
SetWindowsHookEx(
    WH_KEYBOARD_LL,
    ...
)
```

可以安装低级键盘 Hook。微软文档明确说明 `WH_KEYBOARD_LL` 用于监视低级别键盘输入，而且属于全局 Hook。([Microsoft Learn][2])

Rust 可以调用这个 Win32 API。

架构：

```text
Keyboard
   │
   ▼
WH_KEYBOARD_LL
   │
   ├── Ctrl
   │
   ├── G
   │
   └── 判断组合键
          │
          ▼
     Ctrl + G
          │
          ▼
 GetForegroundWindow()
```

然后：

```rust
if ctrl_down && key == VK_G {
    handle_ctrl_g();
}
```

---

# 五、但是这里有一个非常重要的问题

你不能简单地：

```text
Ctrl+G
↓
GetForegroundWindow()
↓
认为它是文件选择框
```

因为用户可能在：

```text
Chrome
VSCode
Explorer
Notepad
Photoshop
各种软件
```

里面按 Ctrl+G。

所以需要判断：

> **当前前台窗口是不是一个 Open/Save File Dialog。**

---

# 六、怎么判断 File Dialog？

Windows 常见文件选择框大概是：

```text
Open
Save As
Choose File
Select File
```

底层可能来自：

### 1. IFileDialog

现代 Windows 应用大量使用：

```cpp
IFileDialog
```

微软把它定义为 Windows Common File Dialog 的接口，提供：

```text
GetFolder()
GetCurrentSelection()
SetFolder()
SetDefaultFolder()
...
```

等操作。([Microsoft Learn][3])

### 2. 老式 Common Dialog

例如：

```text
#32770
```

窗口类。

### 3. 自定义 File Picker

例如：

```text
Chrome
Electron
Qt
WPF
某些游戏
某些 IDE
```

可能根本不是标准 Windows File Dialog。

所以这里实际上要做一个：

```text
DialogDetector
```

---

# 七、最漂亮的情况：标准 IFileDialog

如果当前程序使用的是标准 Windows File Dialog，那么事情非常舒服。

因为 Windows 本身就提供：

```cpp
IFileDialog::SetFolder(IShellItem*)
```

微软文档明确说明：

> 如果在 Dialog 已经显示的时候调用 `SetFolder`，会使 Dialog 导航到指定文件夹。([Microsoft Learn][4])

也就是说最终核心动作可以是：

```text
Explorer Path
    ↓
D:\Projects\rust
    ↓
SHCreateItemFromParsingName()
    ↓
IShellItem
    ↓
IFileDialog::SetFolder()
    ↓
File Dialog
    ↓
D:\Projects\rust
```

这就是最理想的实现。

---

# 八、Rust 里面大概长这样

如果使用 `windows` crate，概念上：

```rust
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        UI::Shell::*,
    },
};
```

然后：

```rust
let shell_item: IShellItem = SHCreateItemFromParsingName(
    path,
    None,
)?;
```

然后：

```rust
file_dialog.SetFolder(&shell_item)?;
```

这里真正麻烦的是：

> **你如何从“别的应用程序已经打开的 File Dialog HWND”拿到它对应的 `IFileDialog` COM 对象？**

这个问题比 `SetFolder` 本身复杂得多。

---

# 九、这里是整个项目最值得研究的地方

不要误以为：

```rust
HWND
 ↓
IFileDialog
```

可以直接转换。

通常不是这么简单。

`IFileDialog` 是**创建这个 Dialog 的进程内部 COM 对象**。

假设：

```text
VSCode.exe
   │
   └── IFileDialog
           ↑
           │
       File Dialog HWND
```

你的程序：

```text
ListaryClone.exe
```

拿到了：

```text
HWND = 0x123456
```

但不能直接：

```rust
IFileDialog::from(hwnd)
```

因为 COM 对象在另外一个进程。

所以这里会出现一个很重要的设计选择。

---

# 十、方案一：UI Automation

这是我非常推荐你第一版采用的。

Windows 的 UI Automation 可以操作标准文件对话框。

微软现在甚至提供了文件对话框的 UI Automation 示例，明确说明 Open/Save 对话框支持 UIA，并可以找到 Dialog HWND、设置路径等。([Microsoft Learn][5])

你的流程可以变成：

```text
Ctrl+G
   ↓
找到当前 Dialog HWND
   ↓
UI Automation
   ↓
找到路径输入控件
   ↓
SetValue("D:\\Projects\\rust")
   ↓
Enter
```

实际上就等价于用户手动：

```text
Alt + D
D:\Projects\rust
Enter
```

这种方式的优点是：

### 不需要注入 DLL

这是非常重要的。

你的程序可以完全是：

```text
listary-rs.exe
```

而不需要：

```text
xxx.dll
```

注入：

```text
explorer.exe
chrome.exe
code.exe
```

---

# 十一、所以我推荐的 Rust 架构是

```text
                     ┌──────────────────┐
                     │  Rust Background  │
                     │      Process     │
                     └────────┬─────────┘
                              │
             ┌────────────────┼────────────────┐
             │                │                │
             ▼                ▼                ▼
      Keyboard Hook     Explorer Tracker   Dialog Detector
             │                │                │
             │                │                │
          Ctrl+G          Current Path       HWND
             │                │                │
             └────────────────┼────────────────┘
                              │
                              ▼
                       Path Resolver
                              │
                              ▼
                       Dialog Navigator
                              │
                 ┌────────────┴────────────┐
                 │                         │
                 ▼                         ▼
            UI Automation             IFileDialog
                 │                         │
                 └────────────┬────────────┘
                              ▼
                       Navigate Folder
```

---

# 十二、还有一个非常有意思的地方：Explorer 路径监听

其实你甚至不需要每次 Ctrl+G 才去查询 Explorer。

可以维护一个：

```rust
HashMap<HWND, ExplorerState>
```

例如：

```text
Explorer #1
    HWND = 0x1001
    Path = C:\Users\David\Downloads
    Active = false

Explorer #2
    HWND = 0x1002
    Path = D:\Work\Rust
    Active = true
```

当 Explorer：

```text
D:\Work\Rust
```

切换到：

```text
D:\Work\Rust\listary-clone
```

你的 tracker 更新：

```text
0x1002
    ↓
D:\Work\Rust\listary-clone
```

这样 Ctrl+G 时：

```text
Ctrl+G
 ↓
当前 File Dialog
 ↓
最近活动 Explorer
 ↓
D:\Work\Rust\listary-clone
```

几乎就是 Listary 的行为。

---

# 十三、Listary 自己是不是这样做的？

这里需要特别区分：

**我们可以确定行为，但不能从公开资料断言 Listary 内部具体用了哪个 Windows API。**

Listary 官方只展示了最终能力；用户社区也有人专门询问它是不是监控 Explorer 当前目录，Listary 并没有公开一个完整的内部实现方案。([Listary Discussions][6])

但是从 Windows API 能力来看：

```text
Explorer path tracking
+
global keyboard interception
+
File Dialog detection
+
dialog navigation
```

完全可以实现这种效果。

而且已经有人做了类似的开源项目。例如 `FolderJump` 明确把自己的目标描述为参考 Listary `Ctrl+G`，并使用 COM 获取 Windows Explorer 当前路径。([GitHub][7])

这个项目非常值得你拿来研究。

---

# 十四、Rust 技术栈我会这么选

如果你真准备开发，我建议：

### Win32

```toml
windows = "..."
```

负责：

```text
Win32 API
COM
Shell
Window
Keyboard Hook
UI Automation
```

---

### Explorer

```text
ShellWindows
IShellFolder
IShellItem
IWebBrowserApp
```

主要负责：

```text
Explorer HWND
    ↓
当前目录
```

---

### 键盘

```text
SetWindowsHookExW
WH_KEYBOARD_LL
```

负责：

```text
Ctrl + G
```

微软文档也特别提醒，全局 Hook 是共享资源，应该谨慎使用；低级键盘 Hook 的回调还需要及时处理，否则会影响输入链。([Microsoft Learn][2])

所以不要在 Hook callback 里面做：

```rust
COM query
filesystem
UI Automation
网络操作
```

而应该：

```text
Hook callback
      │
      └── post event
             ↓
       worker thread
             ↓
       actual processing
```

---

# 十五、文件 Dialog 导航建议分三级

如果你希望最终做到接近商业软件的兼容性，我会设计成：

```text
Navigator
   │
   ├── Strategy 1: IFileDialog
   │
   ├── Strategy 2: UI Automation
   │
   ├── Strategy 3: Keyboard fallback
   │
   └── Strategy 4: Application-specific
```

例如：

### Strategy 1

```text
IFileDialog
```

最干净。

### Strategy 2

```text
UI Automation
```

兼容性好。

### Strategy 3

模拟：

```text
Ctrl+L
type path
Enter
```

或者找到路径 Edit 控件。

### Strategy 4

针对：

```text
Chrome
Electron
Qt
某些特殊应用
```

做兼容。

---

# 十六、甚至可以不用 UI Automation 做第一版

如果你只是想做一个 MVP，我反而建议你：

```text
Explorer Tracker
+
WH_KEYBOARD_LL
+
File Dialog HWND Detection
+
SendInput
```

先跑起来。

流程：

```text
Explorer:

D:\workspace\project
```

然后：

```text
打开 VSCode → File → Open
```

按：

```text
Ctrl+G
```

你的程序：

```text
GetForegroundWindow()
        ↓
判断 #32770
        ↓
SendInput(...)
        ↓
输入 D:\workspace\project
        ↓
Enter
```

这样很快就能做出：

> **80% 的 Listary Ctrl+G 体验。**

然后再逐步替换成：

```text
SendInput
   ↓
UI Automation
   ↓
IFileDialog
```

---

# 十七、但有一个坑：现代 Windows 的 Explorer

如果你做这个项目，我建议你不要把：

```text
Explorer = Shell_TrayWnd / CabinetWClass
```

这种窗口类判断写死。

应该从：

```text
ShellWindows
```

去识别 Explorer。

因为你真正关心的是：

```text
IShellWindows
     ↓
InternetExplorer / Explorer
     ↓
LocationURL
```

而不是单纯：

```text
HWND class == CabinetWClass
```

这样会稳很多。

---

# 十八、最终可以做到什么程度？

如果认真做，Rust 版本可以做到：

```text
                   Listary-like
                        │
        ┌───────────────┼────────────────┐
        │               │                │
        ▼               ▼                ▼
     Explorer        File Dialog      Shortcut
       Path              │              Ctrl+G
        │                │                │
        └────────────┬───┴────────────────┘
                     │
                     ▼
              Jump to Folder
```

甚至可以进一步实现 Listary 的其它体验：

```text
Ctrl+G
    ↓
最近使用目录
    ↓
搜索目录
    ↓
Explorer 当前目录
    ↓
固定目录
```

以及：

```text
Double Ctrl
```

之类的快速搜索入口。

---

# 十九、如果让我来做，我会这样拆 Rust 项目

```text
listary-rs/
│
├── src/
│   ├── main.rs
│   │
│   ├── keyboard/
│   │   ├── mod.rs
│   │   └── hook.rs
│   │
│   ├── explorer/
│   │   ├── mod.rs
│   │   ├── windows.rs
│   │   └── tracker.rs
│   │
│   ├── dialog/
│   │   ├── mod.rs
│   │   ├── detector.rs
│   │   ├── file_dialog.rs
│   │   └── ui_automation.rs
│   │
│   ├── navigation/
│   │   ├── mod.rs
│   │   ├── i_file_dialog.rs
│   │   ├── uia.rs
│   │   └── keyboard.rs
│   │
│   └── state/
│       └── recent_paths.rs
│
└── Cargo.toml
```

核心数据结构甚至可以很简单：

```rust
pub struct ExplorerLocation {
    pub hwnd: HWND,
    pub path: PathBuf,
    pub last_active: Instant,
}
```

然后：

```rust
pub trait DialogNavigator {
    fn can_handle(&self, hwnd: HWND) -> bool;

    fn navigate(
        &self,
        hwnd: HWND,
        path: &Path,
    ) -> Result<()>;
}
```

实现：

```rust
struct FileDialogNavigator;
struct UiaNavigator;
struct KeyboardNavigator;
```

最终：

```rust
for navigator in navigators {
    if navigator.can_handle(dialog_hwnd) {
        navigator.navigate(dialog_hwnd, &path)?;
        break;
    }
}
```

这个架构会非常舒服。

---

## 最关键的技术路线

如果你的目标就是**“用 Rust 仿一个 Listary 的 Ctrl+G”**，我建议不要一上来研究 DLL 注入。

先做：

```text
① Explorer 当前目录获取
        ↓
② WH_KEYBOARD_LL 捕获 Ctrl+G
        ↓
③ 获取当前 File Dialog HWND
        ↓
④ UI Automation 设置路径
        ↓
⑤ Enter
```

这五步跑通以后，再研究：

```text
IFileDialog
```

来提高标准文件对话框的可靠性。

其中 **① 和 ④ 是整个项目最核心的两个技术点**。

而且这条路线在 Rust 上是现实可行的，不需要 C++ 才能完成。

如果你愿意继续往下做，我可以直接给你设计一个**能编译运行的 Rust MVP**：用 `windows` crate 实现 **Explorer 当前路径监听 + 全局 Ctrl+G Hook + 自动识别 Open/Save 对话框 + 跳转目录**，先把 Listary 这个核心功能跑起来。

[1]: https://www.listary.net/feature/quick-save-and-open?utm_source=chatgpt.com "Listary - 快速保存＆打开 - Windows 文件搜索工具｜免费下载"
[2]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw?utm_source=chatgpt.com "SetWindowsHookExW function (winuser.h) - Win32 apps | Microsoft Learn"
[3]: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifiledialog?utm_source=chatgpt.com "IFileDialog (shobjidl_core.h) - Win32 apps | Microsoft Learn"
[4]: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifiledialog-setfolder?utm_source=chatgpt.com "IFileDialog::SetFolder (shobjidl_core.h) - Win32 apps | Microsoft Learn"
[5]: https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/ui-automation?utm_source=chatgpt.com "UI Automation - Windows apps | Microsoft Learn"
[6]: https://discussion.listary.com/t/ctrl-g-function/5880?utm_source=chatgpt.com "Ctrl G function - General - Listary Discussions"
[7]: https://github.com/roverway/folder-jump?utm_source=chatgpt.com "GitHub - roverway/folder-jump: AutoHotkey v2 path switcher for Windows, inspired by Listary `Ctrl+G` · GitHub"
