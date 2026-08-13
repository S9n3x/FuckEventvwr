# Saga - Windows 事件日志分析工具

![Saga](https://socialify.git.ci/Synex93/Saga/image?custom_language=Rust&font=Inter&forks=1&issues=1&language=1&name=1&owner=1&pulls=1&stargazers=1&theme=Light)

## 关于重构
原 Go 版本存在以下问题：
- 解析速度过于缓慢，未找到合适的替换方案
- 由于解析缓慢，CLI 方案需要缓存机制，同样未找到合适方案

---

## 使用方法

```text
Usage: Saga.exe [OPTIONS] [COMMAND]

Commands:
  Application        程序活动与安装, EventID: 1000, 1001, 1002, 1026, 1033, 1034, 1040, 1042, 11707, 11708, 11724
  MSSQL              MSSQL 安全事件, EventID: 15281, 15457, 17806, 17832, 17836, 18452, 18454, 18456, 18470, 33205
  Authentication     验证相关信息, EventID: 4624, 4625, 4648, 4672, 4740, 4768, 4771, 4776
  Session            会话生命周期, EventID: 4634, 4647, 21, 22, 23, 24, 25, 40, 1149
  AccountManagement  账户管理, EventID: 4720, 4722, 4723, 4724, 4725, 4726, 4738, 4740, 4767
  ServiceControl     服务管理, EventID: 7034, 7035, 7036, 7040, 7045
  ScheduledTask      计划任务, EventID: 4698, 4699, 4700, 4701, 4702, 106, 140, 141, 200, 201
  PowerShell         PowerShell 执行日志, EventID: 4103, 4104, 4105, 4106, 400, 403, 600

Options:
  -p, --path <PATH>  指定日志路径，默认为当前系统日志存放位置
  -o, --out <OUT>    输出模式 [default: excel] [possible values: csv, excel]
  -j, --jobs <JOBS>  并发线程数，默认 4 (生产环境安全值) [default: 4]
  -h, --help         显示帮助信息
  -V, --version      显示版本信息
```

子命令支持**前缀匹配**，无需输入完整命令名：

```powershell
# 以下写法等价
Saga.exe Authentication
Saga.exe Authen
Saga.exe Auth
```

### 参数顺序与路径引号

`-p`、`-o` 和 `-j` 是全局参数，可以放在模块前或模块后：

```powershell
Saga.exe -p "D:\Case Files\Windows 日志" Authentication
Saga.exe Authentication -p "D:\Case Files\Windows 日志"
Saga.exe Auth -o csv -j 8 -p "D:\案件\导出的事件日志"
```

路径支持中文、英文及空格。PowerShell/CMD 中带空格的路径请优先使用**半角双引号**：

```powershell
Saga.exe Auth -p "D:\Case Files\Windows 日志"
```

程序也会兼容复制粘贴时带入的全角/弯引号（`“路径”`、`‘路径’`）；但由于终端对全角引号的分词规则不一致，标准半角双引号仍是最稳定的写法。

### 按需请求管理员权限

Saga 默认以普通权限启动，不会一开始就弹出 UAC：

- 指向当前账户可读取的普通目录时，直接解析，不请求管理员权限；
- 默认读取 `C:\Windows\System32\winevt\Logs` 或其他受保护目录时，程序会先实际尝试读取所需 EVTX 文件；
- 仅当读取被 Windows 拒绝时，程序才自动请求一次管理员权限并保留原有参数重新启动；
- 取消 UAC 或提权后仍无权限时会输出错误，不会循环弹窗。

---

## 示例

```powershell
# 分析当前系统的登录验证日志（必要时自动请求管理员权限）
Saga.exe Authentication

# 指定可读取的导出日志目录（不需要管理员权限）
Saga.exe Authentication -p "D:\Exported EVTX"

# 分析 PowerShell 执行记录并导出 CSV
Saga.exe PowerShell -p "D:\案件\事件日志" -o csv
```

---

## 支持的日志模型

| 模型                | 日志文件                               | 说明                                 |
| ------------------- | -------------------------------------- | ------------------------------------ |
| `Application`       | Application.evtx                       | 程序异常、安装活动                   |
| `MSSQL`             | Application.evtx                       | MSSQL 认证、协议、配置与审计事件     |
| `Authentication`    | Security.evtx                          | 登录成功/失败、NTLM/Kerberos 验证    |
| `Session`           | Security.evtx, TerminalServices\*.evtx | RDP 会话创建、断连、重连             |
| `AccountManagement` | Security.evtx                          | 账户创建、删除、启用、禁用、密码修改 |
| `ServiceControl`    | System.evtx                            | 服务安装、启动、停止、崩溃           |
| `ScheduledTask`     | Security.evtx, TaskScheduler\*.evtx    | 计划任务创建、修改、执行             |
| `PowerShell`        | PowerShell\*.evtx                      | 脚本块内容、命令执行记录             |

---

## 关于性能
测试环境：Intel Core Ultra 5 338H（12 核 12 线程），默认 4 线程（生产环境安全值），实测数据如下：

| 模型           | 输出格式 | 解析记录数   | 总耗时 | 平均速度           |
| -------------- | -------- | ------------ | ------ | ------------------ |
| Authentication | Excel    | 2,738,874 条 | ~18.5s | **~148,000 条/秒** |
| Authentication | CSV      | 2,738,874 条 | ~13.7s | **~200,000 条/秒** |
| Session        | Excel    | 2,742,760 条 | ~14.5s | **~189,000 条/秒** |
| ScheduledTask  | Excel    | 2,756,432 条 | ~11.3s | **~244,000 条/秒** |

默认 4 线程适用于直接在生产服务器上运行，避免抢占业务 CPU。实测 `-j 12` 相比默认值在所有模型上均无显著提升（差距 <0.2s），当前瓶颈在 evtx 二进制解码阶段——每个 evtx 文件由单线程解码，XML 解析已并行化但并非瓶颈。各模型均以 `Security.evtx`（270 万+ 条记录）为主导，多文件并未带来更多并行解码机会。`-j` 参数预留给后续解码层优化后使用。

构建产物体积：**~2MB**（`cargo build --release`）
相比较之前GO语言版本解析速度“单文件，50w条数据，大概需要40秒左右”，现在可以说非常快了。

---

## 构建

### 普通 Windows 版本

```powershell
cargo build --release
```

构建产物：`target\release\Saga.exe`

### Windows 7 兼容版本

Win7 构建目标需要 nightly 工具链及 `rust-src` 组件：

```powershell
rustup toolchain install nightly --component rust-src
cargo +nightly win7
```

构建产物：`target\x86_64-win7-windows-msvc\release\Saga.exe`

Win7 目标的标准库会在本地从源码构建，因此首次编译耗时较长，后续构建可复用缓存。

---

## 写在后面

输出方面已支持 CSV 和 Excel 两种格式，数值字段（如事件 ID、登录类型）以原生数字类型写入 Excel，可直接用于排序和筛选。

事件解析模板（各模型的字段提取逻辑）目前大部分由 AI 辅助生成，准确性和覆盖度有限，后续重心将转向人工校验解析准确性和补全更多场景的模板。当前性能瓶颈在 evtx 二进制解码（单文件单线程），后续也会探索解码层并行化。

如果你在使用中遇到字段缺失、解析错误或有新的场景需求，欢迎提 ISSUE。
