# Windows 应用索引扫描方案

> 更新时间：2026-09-21
> 适用范围：Windows 应用索引。macOS 扫描逻辑保持原样。
> 相关实现：`src-tauri/src/api/explorer.rs`、`src-tauri/src/api/windows_apps.rs`、`src-tauri/src/utils/database.rs`、`src-tauri/src/api/file.rs`

## 1. 目标

索引展示的是用户可以直接启动的应用，而不是磁盘上所有的 `.exe`。当前方案采用“注册应用优先、文件系统补充”的分层策略：

1. Windows `shell:AppsFolder` 注册应用是主数据源；
2. 当前用户桌面和公共桌面是补充来源；
3. `local_app_search_paths` 用于发现绿色版、便携版应用；
4. 用户手动添加的自定义应用始终保留。

## 2. 数据源与边界

### 2.1 AppsFolder

`AppsFolder` 覆盖传统 Win32 应用、计算器、记事本以及 Microsoft Store/UWP 应用。每条记录统一包含显示名称、启动目标、可选实际 `.exe` 路径和图标来源。

存在 `System.AppUserModel.ID` 时，启动目标使用：

```text
shell:AppsFolder\<AppUserModelID>
```

传统应用如果能够解析出实际 `.exe`，该路径用于图标、搜索和去重；没有普通文件路径的 Store/UWP 应用也不会因此被丢弃。

### 2.2 桌面

当前用户桌面和公共桌面进行非递归扫描，保留 `.exe` 和 `.rdp` 入口。桌面扫描不加入普通文本、ReadMe、图片等文件。

`.url` 不作为应用索引来源，因为它更接近网页书签，不符合应用启动入口的定义。

### 2.3 用户配置目录

`local_app_search_paths` 只作为便携应用补充来源：

- 只收录明确可启动的 `.exe`；
- 使用 `local_app_search_exclude_paths` 排除文件系统目录；
- 不扫描全盘、Windows 目录或整个 Program Files；
- 不把目录中的每个 `.exe` 都视为用户应用。

路径排除规则只作用于文件系统扫描，不能用来删除没有普通路径的 Store/UWP 应用。

## 3. 已注册应用目录抑制

如果 AppsFolder 已经注册：

```text
D:\App\QQMusic\QQMusic.exe
```

扫描器会把其直接父目录 `D:\App\QQMusic` 视为高可信应用目录。便携目录扫描时，该目录及其子目录下的 `.exe` 不再作为独立自动应用加入索引，从而避免出现：

```text
QMDriverHelperx64
QMWeiyun
QQMusicUp
qmbrowser
StartDesktopProjection32
```

该规则只影响自动发现，不影响 AppsFolder 主应用、自定义应用或没有实际 `.exe` 路径的 Store/UWP 应用。

不会按品牌根目录整体排除。例如：

```text
D:\App\JetBrains\PyCharm 2026.1
D:\App\JetBrains\RustRover2026.1
```

只处理每个已注册产品的具体目录，不排除 `D:\App\JetBrains`，因此多个 JetBrains 产品仍能分别保留。路径比较统一斜杠并忽略大小写，同时检查目录边界，避免 `QQMusic` 错误匹配 `QQMusic2`。

## 4. 过滤规则

### 4.1 通用过滤

以下条目不会进入自动应用索引：

- 名称为空；
- 启动目标为空；
- 扩展名不符合来源规则；
- 无法形成有效启动标识；
- 明显的卸载器、更新器、运行库、服务或后台辅助进程。

图标读取失败不会删除应用，会回退为空图标或默认图标。

### 4.2 辅助程序

当前规则覆盖常见命名：

```text
agent external service svr helper launcher swap crashpad updater
uninstaller uninstall uninst unins readme old runtime worker bootstrap
```

并针对已知组件覆盖：

```text
QQMusicUp
QMDriverHelperx64
QMDesktopAnimation
DesktopDynamicLyric
StartDesktopProjection32
qmbrowser
QMWeiyun
Quark PWA Launcher
```

规则优先使用完整 token、明确后缀和已确认名称，避免仅凭模糊子串误删 `ServerManager` 等正常应用。

### 4.3 标题

标题用于展示和搜索，不直接等同于文件名。常见文件扩展名不会作为应用标题的一部分展示，例如 `quark.exe` 应显示为 `quark`。但同名而启动目标不同的应用不能仅凭名称合并。

## 5. 去重与优先级

来源优先级：

1. 用户手动添加的自定义应用；
2. AppsFolder 注册应用；
3. 桌面补充入口；
4. 配置目录发现的便携应用。

自动扫描首先按规范化启动标识去重：文件路径统一斜杠并忽略大小写，Shell/AppUserModel 标识也忽略大小写。同名但启动目标不同的应用保留多条。

便携应用另外按产品族去重，用于处理：

```text
D:\App\Quark\quark.exe
D:\App\Quark\7.1.7.975\quark.exe
D:\App\Quark\7.2.0.992\quark.exe
```

版本目录视为同一产品族，通常只保留优先的主程序。产品名称仍参与区分，所以 `PyCharm` 和 `RustRover` 不会因共享 `JetBrains` 父目录而合并。

## 6. 重建索引

重建索引时，数据库事务会先删除自动发现记录：

```sql
DELETE FROM app_index WHERE is_custom = 0
```

随后写入本次扫描结果。`is_custom = 1` 的手动应用不删除、不覆盖；写入失败时事务回滚。因而移除配置目录并重建后，旧的自动记录应消失。

扫描完成会输出扫描/写入日志并发送完成事件。若前端当前结果列表没有重新加载，界面可能短暂显示旧结果，但数据库已经是本次重建结果。

## 7. 启动与图标

- 普通 `.exe`：沿用普通 Windows 启动路径；
- `shell:AppsFolder\...`：通过 Windows Shell 打开，不能传给 `CreateProcessW`；
- `.rdp`：沿用 Windows 文件关联；
- `.url`：不进入应用索引。

图标优先使用 AppsFolder 属性提供的包图标，其次使用关联图标或 PE 图标。属性读取失败、图标路径无扩展名或图标损坏都只影响图标，不影响应用记录。

## 8. 典型结果

### QQ 音乐

保留：

```text
QQ音乐    shell:AppsFolder\...
```

过滤内部组件：

```text
DesktopDynamicLyric
QMDesktopAnimation
QMDriverHelperx64
QMWeiyun
QQMusicUninst
QQMusicUp
StartDesktopProjection32
StartDesktopProjectionForXP
qmbrowser
```

### Quark

多个版本目录的 `quark.exe` 按产品族去重，同时过滤 `quark_swap_util`、`quark_pwa_launcher` 和 `old_quark`。

### JetBrains

`PyCharm 2026.1` 和 `RustRover2026.1` 作为两个独立应用保留。

## 9. 验证要求与限制

应在真实 Windows 环境验证：

- 计算器、记事本和至少一个 Store/UWP 应用可搜索、显示图标并启动；
- 开始菜单传统 Win32 应用正常；
- 桌面 `.exe`、`.rdp` 保留，`.url` 不出现；
- 更新器、卸载器、服务、ReadMe 和辅助进程不出现；
- 配置目录移除后重建，旧自动记录消失；
- 自定义应用重建后仍保留；
- 坏 Shell 属性和图标失败不会中断扫描。

已执行静态检查：

```text
cargo fmt
git diff --check
```

完整 `cargo check` 若被本机 Cargo target 锁文件权限阻塞，只能记录为环境限制，不能视为编译验证成功。

## 10. 维护原则

新增规则时优先级如下：

1. 优先依赖 AppsFolder 注册信息；
2. 其次使用已注册应用实际目录抑制；
3. 再增加明确的辅助程序规则；
4. 最后才考虑模糊名称规则。

禁止通过扫描全盘、按品牌根目录整体排除、仅凭同名合并或图标失败删除应用来解决噪音问题。

## 9. AppsFolder 变化后的自动刷新

Windows 运行期间通过 Shell 变化通知监听 `shell:AppsFolder`。通知只作为“应用目录可能变化”的信号，不直接根据单个事件推导数据库增删。

连续通知经过 2 秒防抖后串行执行现有完整应用扫描；扫描期间的新通知会在本轮完成后再次触发。AppsFolder 枚举失败时不执行自动记录替换，保留数据库中的旧索引，避免把临时 COM 或 Shell 错误误判为“系统没有应用”。

当前自动刷新不监听 Program Files、卸载注册表、`PackageCatalog` 或便携应用目录；这些来源是否需要补充，应以 Windows 实机安装、卸载覆盖测试结果为依据。
