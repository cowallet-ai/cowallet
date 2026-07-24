# 登录/引导流程改造为全路由导航 (COW-26)

- **Issue**: [COW-26](https://linear.app/clawmint/issue/COW-26/将登录引导流程改造为全路由导航)
- **日期**: 2026-07-24
- **范围**: `mobile/` Flutter 客户端

## 目标

把登录/引导流程从「单 `StatefulWidget` + `_Stage` 枚举内部状态机」改造为
**基于 Navigator 的全路由导航**：每个阶段是真实路由，返回/侧滑手势语义
可控，杜绝在关键阶段（尤其 DKG 进行中的 `creating`）被误左滑退出，同时
保留「杀掉 App 后回到当时所在步骤」的断点续跑能力。

## 现状

- 整个流程是 `mobile/lib/onboarding/onboarding_flow.dart` 里**一个**
  `_OnboardingFlowState`，靠 `_Stage` 枚举 + `setState` + `AnimatedSwitcher`
  切换 10 个阶段界面，用自维护的 `_history` 列表模拟返回栈。
- 各阶段**不是** Navigator route；流程内**没有任何** `PopScope`/`WillPopScope`
  守卫，系统返回手势作用于整个 onboarding route。
- `creating`（DKG 进行中）缺乏防误退保护。
- `_restoreStep` 从 `SecureStorage.keyOnboardingStep` 恢复步骤，但其
  `stageOrder` 列表把 `bio, name` 排在 `backup` **之前**，与真实运行顺序
  （`creating → backup → bio → name`）不一致 —— 恢复后的返回历史已有偏差。
- 真实运行顺序：`hero/intro`（同一 widget 内 PageView 2 页）→ `email` →
  `emailOtp` → `creating` → `backup` → `bio` → `name` → `ready` → `persona`
  → `pushReplacementNamed('/')`。

## 跨阶段共享状态盘点

以下状态不在 AppState/Services 中，改造后需集中到控制器：

- `email` 字符串（email → otp → register 调用链）
- `forceRegister` + `backupShardHash`（重新注册路径写入，OTP 校验读取）
- `backupSkipped` / `backupDone`（backup 阶段写入，`finish` 时读取）

已在别处、**无需**穿透传递的：钱包地址、用户名、persona 走
`CowalletApp.of(context)` 应用态；备份分片走 service 层 + SecureStorage。

## 架构决策：嵌套子 Navigator（方案 A）

`OnboardingFlow` 内部托管**自己的子 `Navigator`**，10 个阶段是这个子导航栈
里的页面。全局路由表 `app_router.dart` **保持只有一条 `/onboarding`**。

**为什么不是顶层路由**：断点续跑要求"重建整条返回栈"，Flutter 的
`Navigator.onGenerateInitialRoutes` 天生服务于此，放在子 Navigator 里最干净；
根 Navigator 启动逻辑已很复杂（版本检查、钱包检查、force-upgrade），再塞入
onboarding 栈重建会更乱。跨阶段共享状态也能用子树 `InheritedWidget` 干净包住，
无需逐跳穿透路由参数或引入全局单例。

> **对 issue 范围的有意偏离（已与需求方确认接受）**：issue 原文写"在
> `app_router.dart` 新增各阶段路由常量与 `onGenerateRoute` case"。因采用子
> Navigator，阶段路由名改为放在 onboarding 模块内部（`onboarding/routes.dart`），
> 全局路由表不新增阶段路由。

## 组件结构

把巨型 `StatefulWidget` 拆成 **控制器 + 一组纯视图页面**：

- **`OnboardingController`**（新）—— "大脑"。持有跨阶段共享状态、所有
  `Services.*` / `AuthApi` / `ShardsApi` 调用、步骤持久化、DKG / 备份 /
  生物识别等副作用逻辑，并暴露阶段间导航方法（`goToEmail`、`goToOtp`、
  `startCreating`、`onDkgSuccess`…）。**这是唯一的可注入接缝**：生产用真实
  实现，测试注入 fake。持有子 Navigator 的 `GlobalKey<NavigatorState>`。
- **`OnboardingScope`**（新，`InheritedWidget`）—— 向子树提供 controller，
  阶段页面通过 `OnboardingScope.of(context)` 取用。
- **`OnboardingFlow`**（改）—— 瘦身为：`OnboardingScope` 包裹一个子
  `Navigator`（`onGenerateRoute` + `onGenerateInitialRoutes`）。
- **阶段页面**（新，`onboarding/stages/*.dart`）—— 每个是独立
  `StatefulWidget`，只保留本地 UI 状态（`TextEditingController`、spinner
  开关、本地错误串），业务动作全部委托给 controller。`hero` 与 `intro`
  合并为**一条路由内的 2 页 PageView 轮播**。

好处：每个阶段文件小而聚焦，controller 集中副作用，视图可独立测试。

## 导航栈模型

子 Navigator 使用 **`CupertinoPageRoute`**（原生滑动过渡，iOS 边缘左滑返回
手势天然可用）。栈的分段与守卫：

```
DKG 前 (可自由左滑返回):   hero → email → otp
DKG 中 (canPop:false):     creating              ← 左滑/系统返回全拦截
DKG 后 (backup 是硬 floor): backup                ← canPop:false
DKG 后 (组内可左滑返回):    bio → name → ready → persona
```

**边界模型（已确认）**：`creating` 与 `backup` 均 `PopScope(canPop:false)`。
DKG 完成 = 一道硬边界：钱包已建好，左滑退回 DKG 前的步骤会进入非法状态
（拿旧 OTP 重新注册），故 `backup` 不可返回。issue 完成标准里"backup 可左滑
返回"的措辞与此矛盾，以本边界模型为准。

**关键转场**：

- `otp → creating`：`push`（`creating` 的 `PopScope` 拦截手势）。
- DKG 成功 → `pushAndRemoveUntil(backup)`：清掉整个 DKG 前的栈。
- `backup → bio`：`pushReplacement`，令 `bio` 成为"DKG 后可返回组"的新栈底
  （`bio` 左滑退不回 `backup`；`bio↔name↔ready↔persona` 互相可返回）。
- `persona` / `finish → '/'`：用**根** Navigator `pushReplacementNamed('/')`。

## 持久化与断点续跑

`onGenerateInitialRoutes` 启动时从 `SecureStorage.keyOnboardingStep` 读取步骤
并重建整条栈；controller 在每次进入新阶段时写入该键，`finish` 时删除。

| 存的步骤 | 重建的栈 |
|---|---|
| 无 / `hero` | `[hero]` |
| `email` | `[hero, email]` |
| `emailOtp` | `[hero, email, otp]` |
| **`creating`** | `[creating]`（`initState` 中 `canResume()` 自动续跑 DKG） |
| `backup` | `[backup]` |
| `bio` | `[bio]` |
| `name` | `[bio, name]` |
| `ready` | `[bio, name, ready]` |
| `persona` | `[bio, name, ready, persona]` |

**`creating` 续跑（方案 a，已确认）**：杀在 DKG 进行中，下次仍落在
`creating`，`initState` 调 `MpcSessionManager.canResume()` 自动续跑/重跑 DKG。
这**替换**了旧 `_restoreStep` 里"遇到 `creating` 就不恢复"的逻辑。新恢复表按
真实流程重建，天然修正旧 `stageOrder` 的 `bio/name`↔`backup` 顺序 bug。

## 两个恢复相关细节

- **`/recovery` 路由**：现从 `hero` 与 email 弹窗 `pushNamed('/recovery')`。
  改造后它仍是**顶层**路由，必须用
  `Navigator.of(context, rootNavigator: true).pushNamed(...)`，否则会错误地
  压进 onboarding 子栈、可被自由左滑退出。
- **`keyOnboardingStep` 写入时机**：进入每个阶段路由时由 controller 写入；
  `finish` 时 `delete`。语义与现状一致。

## 测试方案（满足 issue 强制要求）

现状阻碍：阶段直接调 `Services.*`（`static late` 字段）与 `AuthApi`/
`ShardsApi` 静态方法，在 `flutter test` host 中无法干净伪造（这正是现有
`widget_test.dart` 只能冒烟测试的原因）。改造后所有副作用集中在 controller，
即可注入 fake。

- **Fake controller**：`FakeOnboardingController` 的动作方法即时成功、不碰
  `Services`/网络，但真实驱动子 Navigator。经 `OnboardingScope` 注入。
- **测试 1 —— 阶段前进/返回**：驱动 controller 导航（或点按钮），断言可见
  阶段正确切换，`pop` 后回到上一阶段。
- **测试 2 —— `creating` 拦截**：子 Navigator 初始栈设为 `[creating]`，触发
  系统返回（`didPopRoute`），断言路由未退出、`canPop == false`。

## 改动范围

**改**：
- `mobile/lib/onboarding/onboarding_flow.dart` —— 瘦身为子 Navigator 宿主。
- （`main.dart` 挂载方式不变：`initialRoute` 仍是 `/onboarding`。）

**新增**：
- `mobile/lib/onboarding/controller.dart`
- `mobile/lib/onboarding/scope.dart`
- `mobile/lib/onboarding/routes.dart`（阶段路由名常量 + 转场，模块内部）
- `mobile/lib/onboarding/stages/*.dart`（各阶段页面：hero、email、otp、
  creating、backup、bio、name、ready、persona）
- `mobile/test/onboarding/`（2 个 widget 测试 + fake controller）

## 完成标准

- `cd mobile && flutter analyze` 零 error / 零 warning。
- `cd mobile && flutter test` 全绿；新增测试断言 `creating` 路由触发系统返回
  不退出流程（`canPop == false`）。
- 各阶段均为独立 Navigator route，不再依赖 `_Stage` + `setState` 切界面；
  `AnimatedSwitcher` 状态机移除（`hero/intro` 内部 PageView 保留）。
- 手动验证：iOS 上 email/otp 阶段左滑 = 返回上一步；`creating`、`backup`
  阶段左滑无效；DKG 后 `bio↔name↔ready↔persona` 可左滑返回；完成后 `/`
  首页不可左滑回登录。
- 无新增环境变量/配置。

## 非目标 (YAGNI)

- **不做**阶段深链（deep-link）—— issue 完成标准未要求，子 Navigator 方案
  下也无对应入口。
- **不改**各阶段的业务逻辑 / 网络协议 / UI 文案（纯导航结构重构）。
- **不做**无关重构。
