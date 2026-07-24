# Onboarding Route-Navigation Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the onboarding flow from a single `StatefulWidget` + `_Stage` enum state machine into a nested-`Navigator` route stack, so back/swipe gestures map to logical steps and `creating`/`backup` are guarded against accidental exit.

**Architecture:** `OnboardingFlow` hosts a child `Navigator`; each stage is a `CupertinoPageRoute` inside it. An `OnboardingController` (BuildContext-free, injectable) owns cross-stage state, persistence, navigation, and all `Services`/`AuthApi`/`ShardsApi` calls; stages are pure view widgets that read it via an `OnboardingScope` `InheritedWidget`. The initial stack is rebuilt from `SecureStorage.keyOnboardingStep` via `onGenerateInitialRoutes`.

**Tech Stack:** Flutter 3.41.9, Dart, `flutter_test`, `flutter_secure_storage`, existing MPC/Auth services.

## Convention for this plan

New structural/pure/test files (Tasks 1, 2, 3, 8, 9) are given **complete code**. The stage-widget extractions (Tasks 4–7) are **mechanical moves** of existing markup from `onboarding_flow.dart`; those steps give exact source line ranges plus the precise call-site rewrites (the deltas), not 1400 verbatim lines. The source file is the ground truth for the copied markup.

## Global Constraints

- `cd mobile && flutter analyze` → zero error / zero warning (hard gate every task).
- No new env vars / config.
- Persisted step strings in `SecureStorage.keyOnboardingStep` MUST keep their existing `.name` values (`email`, `emailOtp`, `creating`, `backup`, `bio`, `name`, `ready`, `persona`) for continuity with installed apps.
- Do NOT change business logic, network protocol, or UI copy — navigation structure only.
- `/recovery` stays a **root** route; push it via `Navigator.of(context, rootNavigator: true)`.
- Keep the app compiling and `flutter analyze`-clean between tasks: build new files first; swap `onboarding_flow.dart` only in Task 8.

## File Structure

- Create `mobile/lib/onboarding/routes.dart` — `OnboardingStep` enum, `OnboardingRoutes` name constants, `initialStackFor()` pure fn.
- Create `mobile/lib/onboarding/scope.dart` — `OnboardingScope` InheritedWidget.
- Create `mobile/lib/onboarding/controller.dart` — `OnboardingController`.
- Create `mobile/lib/onboarding/stages/shared.dart` — shared stage widgets (top bar, headings, buttons…).
- Create `mobile/lib/onboarding/stages/*_stage.dart` — 9 stage pages.
- Modify `mobile/lib/onboarding/onboarding_flow.dart` — sub-Navigator host (Task 8).
- Create `mobile/test/onboarding/fake_controller.dart`, `flow_navigation_test.dart`, `creating_guard_test.dart`, `test/onboarding/routes_test.dart`, `test/onboarding/scope_test.dart`.

---
### Task 1: Route names + initial-stack rebuild (pure, unit-tested)

**Files:**
- Create: `mobile/lib/onboarding/routes.dart`
- Test: `mobile/test/onboarding/routes_test.dart`

**Interfaces:**
- Produces:
  - `enum OnboardingStep { hero, email, emailOtp, creating, backup, bio, name, ready, persona }`
  - `class OnboardingRoutes` with `static const String` names equal to each step's `.name`.
  - `List<OnboardingStep> initialStackFor(String? savedStep)` — maps a persisted step string to the full route stack to rebuild (per the spec's restore table). Unknown/null → `[hero]`.
  - `OnboardingStep? stepFromName(String? name)`.

- [ ] **Step 1: Write the failing test**

Create `mobile/test/onboarding/routes_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/routes.dart';

void main() {
  group('initialStackFor', () {
    test('null or unknown → [hero]', () {
      expect(initialStackFor(null), [OnboardingStep.hero]);
      expect(initialStackFor(''), [OnboardingStep.hero]);
      expect(initialStackFor('bogus'), [OnboardingStep.hero]);
    });

    test('pre-DKG steps rebuild the full back stack', () {
      expect(initialStackFor('email'), [OnboardingStep.hero, OnboardingStep.email]);
      expect(initialStackFor('emailOtp'),
          [OnboardingStep.hero, OnboardingStep.email, OnboardingStep.emailOtp]);
    });

    test('creating restores alone (guarded, auto-resumes)', () {
      expect(initialStackFor('creating'), [OnboardingStep.creating]);
    });

    test('backup is a standalone floor', () {
      expect(initialStackFor('backup'), [OnboardingStep.backup]);
    });

    test('post-DKG returnable group rebuilds from bio', () {
      expect(initialStackFor('bio'), [OnboardingStep.bio]);
      expect(initialStackFor('name'), [OnboardingStep.bio, OnboardingStep.name]);
      expect(initialStackFor('ready'),
          [OnboardingStep.bio, OnboardingStep.name, OnboardingStep.ready]);
      expect(initialStackFor('persona'), [
        OnboardingStep.bio,
        OnboardingStep.name,
        OnboardingStep.ready,
        OnboardingStep.persona,
      ]);
    });

    test('route name constants equal enum .name', () {
      expect(OnboardingRoutes.email, OnboardingStep.email.name);
      expect(OnboardingRoutes.emailOtp, OnboardingStep.emailOtp.name);
      expect(OnboardingRoutes.creating, OnboardingStep.creating.name);
    });
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mobile && flutter test test/onboarding/routes_test.dart`
Expected: FAIL — `routes.dart` / `initialStackFor` not defined.

- [ ] **Step 3: Write minimal implementation**

Create `mobile/lib/onboarding/routes.dart`:

```dart
/// The onboarding stages, in true runtime order. Each is a Navigator route.
enum OnboardingStep { hero, email, emailOtp, creating, backup, bio, name, ready, persona }

/// Route-name constants for the onboarding child Navigator. Values equal each
/// [OnboardingStep]'s `.name` so persisted steps map 1:1.
class OnboardingRoutes {
  static const String hero = 'hero';
  static const String email = 'email';
  static const String emailOtp = 'emailOtp';
  static const String creating = 'creating';
  static const String backup = 'backup';
  static const String bio = 'bio';
  static const String name = 'name';
  static const String ready = 'ready';
  static const String persona = 'persona';
}

OnboardingStep? stepFromName(String? name) {
  if (name == null || name.isEmpty) return null;
  for (final s in OnboardingStep.values) {
    if (s.name == name) return s;
  }
  return null;
}

/// Rebuild the child Navigator's initial route stack from a persisted step.
///
/// DKG is a hard boundary: `creating` and `backup` restore as standalone
/// roots (no legal back target), while pre-DKG and the post-DKG returnable
/// group (bio→name→ready→persona) rebuild their full back stack so the swipe
/// gesture steps backwards correctly.
List<OnboardingStep> initialStackFor(String? savedStep) {
  final step = stepFromName(savedStep);
  switch (step) {
    case null:
    case OnboardingStep.hero:
      return const [OnboardingStep.hero];
    case OnboardingStep.email:
      return const [OnboardingStep.hero, OnboardingStep.email];
    case OnboardingStep.emailOtp:
      return const [OnboardingStep.hero, OnboardingStep.email, OnboardingStep.emailOtp];
    case OnboardingStep.creating:
      return const [OnboardingStep.creating];
    case OnboardingStep.backup:
      return const [OnboardingStep.backup];
    case OnboardingStep.bio:
      return const [OnboardingStep.bio];
    case OnboardingStep.name:
      return const [OnboardingStep.bio, OnboardingStep.name];
    case OnboardingStep.ready:
      return const [OnboardingStep.bio, OnboardingStep.name, OnboardingStep.ready];
    case OnboardingStep.persona:
      return const [
        OnboardingStep.bio,
        OnboardingStep.name,
        OnboardingStep.ready,
        OnboardingStep.persona,
      ];
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mobile && flutter test test/onboarding/routes_test.dart`
Expected: PASS (all cases).

- [ ] **Step 5: Analyze + commit**

```bash
cd mobile && flutter analyze lib/onboarding/routes.dart test/onboarding/routes_test.dart
git add mobile/lib/onboarding/routes.dart mobile/test/onboarding/routes_test.dart
git commit -m "feat(onboarding): route names + initial-stack rebuild (COW-26)"
```

---
### Task 2: OnboardingController

**Files:**
- Create: `mobile/lib/onboarding/controller.dart`

**Interfaces:**
- Consumes: `OnboardingStep`, `OnboardingRoutes` (Task 1); `SecureStorage` (`utils/secure_storage.dart`); `Services`, `MpcWalletService`, `MpcSessionManager`.
- Produces `OnboardingController`:
  - Ctor: `OnboardingController({GlobalKey<NavigatorState>? navigatorKey})` — defaults to a fresh key.
  - `final GlobalKey<NavigatorState> navigatorKey;`
  - Shared state fields (public, mutable): `String email = ''`, `bool forceRegister = false`, `String? backupShardHash`, `bool backupSkipped = false`, `bool backupDone = false`.
  - `NavigatorState? get nav => navigatorKey.currentState;`
  - Navigation methods (all `void`, `@visibleForOverriding` so the fake can no-op if needed): `goTo(OnboardingStep)`, `goBack()`, `onDkgSuccess()`, `finish(String walletAddress)`.
  - `Future<void> persistStep(OnboardingStep)` and `Future<void> clearStep()`.
  - `MpcSessionManager get sessionManager` — lazily built from `Services.mpcWallet`.

- [ ] **Step 1: Write the implementation**

Create `mobile/lib/onboarding/controller.dart`:

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter/cupertino.dart';
import '../services/locator.dart';
import '../services/mpc_wallet_service.dart';
import '../services/mpc_session_manager.dart';
import '../utils/secure_storage.dart';
import 'routes.dart';

/// Owns all cross-stage onboarding state, persistence, side-effects, and
/// child-Navigator transitions. BuildContext-free so it can be driven from
/// tests with a fake subclass. Stages read it via [OnboardingScope].
class OnboardingController {
  OnboardingController({GlobalKey<NavigatorState>? navigatorKey})
      : navigatorKey = navigatorKey ?? GlobalKey<NavigatorState>();

  final GlobalKey<NavigatorState> navigatorKey;

  // ---- Cross-stage shared state ----
  String email = '';
  bool forceRegister = false;
  String? backupShardHash;
  bool backupSkipped = false;
  bool backupDone = false;

  NavigatorState? get nav => navigatorKey.currentState;

  MpcSessionManager? _sessionManager;
  MpcSessionManager get sessionManager =>
      _sessionManager ??= MpcSessionManager(Services.mpcWallet);

  MpcWalletService get walletService => Services.wallet as MpcWalletService;

  // ---- Persistence ----
  Future<void> persistStep(OnboardingStep step) =>
      SecureStorage.save(SecureStorage.keyOnboardingStep, step.name);

  Future<void> clearStep() =>
      SecureStorage.delete(SecureStorage.keyOnboardingStep);

  // ---- Transitions ----

  /// Push a stage as the next route and persist it. Used for freely-returnable
  /// forward moves (hero→email→otp, bio→name→ready→persona).
  void goTo(OnboardingStep step) {
    persistStep(step);
    nav?.pushNamed(step.name);
  }

  /// Pop to the previous stage in the child Navigator.
  void goBack() => nav?.maybePop();

  /// DKG completed: clear the pre-DKG stack and land on backup as the new root.
  void onDkgSuccess() {
    persistStep(OnboardingStep.backup);
    nav?.pushNamedAndRemoveUntil(OnboardingRoutes.backup, (r) => false);
  }

  /// backup→bio: replace so bio becomes the returnable-group floor.
  void goToBioFromBackup() {
    persistStep(OnboardingStep.bio);
    nav?.pushReplacementNamed(OnboardingRoutes.bio);
  }

  /// Leave onboarding for the app home via the ROOT navigator.
  Future<void> finish(BuildContext rootContext, String walletAddress) async {
    await clearStep();
    await SecureStorage.save('onboarding_completed_at', DateTime.now().toIso8601String());
    await SecureStorage.save(
        'backup_status', backupSkipped ? 'skipped' : (backupDone ? 'saved' : 'pending'));
    await SecureStorage.save('mpc_address', walletAddress);
    if (walletAddress.isNotEmpty) {
      Services.balance.refresh(walletAddress);
    }
    Navigator.of(rootContext, rootNavigator: true).pushReplacementNamed('/');
  }
}
```

- [ ] **Step 2: Analyze + commit**

Run: `cd mobile && flutter analyze lib/onboarding/controller.dart`
Expected: zero issues.

```bash
git add mobile/lib/onboarding/controller.dart
git commit -m "feat(onboarding): add OnboardingController (state, persistence, nav) (COW-26)"
```

> Note: `finish` takes a root `BuildContext` (not stored) purely to reach the
> root Navigator; all other state is context-free. `goToBioFromBackup` is the
> only replacement transition. `onDkgSuccess` clears the pre-DKG stack.

---
### Task 3: OnboardingScope (InheritedWidget) + test

**Files:**
- Create: `mobile/lib/onboarding/scope.dart`
- Test: `mobile/test/onboarding/scope_test.dart`

**Interfaces:**
- Consumes: `OnboardingController` (Task 2).
- Produces: `class OnboardingScope extends InheritedWidget` with
  `final OnboardingController controller;` and
  `static OnboardingController of(BuildContext) ` (asserts presence).

- [ ] **Step 1: Write the failing test**

Create `mobile/test/onboarding/scope_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/scope.dart';

void main() {
  testWidgets('OnboardingScope.of returns the provided controller', (tester) async {
    final controller = OnboardingController();
    late OnboardingController resolved;
    await tester.pumpWidget(
      OnboardingScope(
        controller: controller,
        child: Builder(builder: (ctx) {
          resolved = OnboardingScope.of(ctx);
          return const SizedBox();
        }),
      ),
    );
    expect(identical(resolved, controller), isTrue);
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mobile && flutter test test/onboarding/scope_test.dart`
Expected: FAIL — `scope.dart` not defined.

- [ ] **Step 3: Write minimal implementation**

Create `mobile/lib/onboarding/scope.dart`:

```dart
import 'package:flutter/widgets.dart';
import 'controller.dart';

/// Provides the [OnboardingController] to the onboarding stage subtree.
class OnboardingScope extends InheritedWidget {
  const OnboardingScope({
    super.key,
    required this.controller,
    required super.child,
  });

  final OnboardingController controller;

  static OnboardingController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<OnboardingScope>();
    assert(scope != null, 'OnboardingScope.of() called with no scope in tree');
    return scope!.controller;
  }

  @override
  bool updateShouldNotify(OnboardingScope oldWidget) =>
      !identical(oldWidget.controller, controller);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mobile && flutter test test/onboarding/scope_test.dart`
Expected: PASS.

- [ ] **Step 5: Analyze + commit**

```bash
cd mobile && flutter analyze lib/onboarding/scope.dart test/onboarding/scope_test.dart
git add mobile/lib/onboarding/scope.dart mobile/test/onboarding/scope_test.dart
git commit -m "feat(onboarding): add OnboardingScope InheritedWidget (COW-26)"
```

---
### Task 4: Shared stage widgets

**Files:**
- Create: `mobile/lib/onboarding/stages/shared.dart`

**Interfaces:**
- Produces top-level functions (moved verbatim from `_OnboardingFlowState`, converted to take `BuildContext context` as first param and `onBack` where they referenced `_goBack`):
  - `Widget obTopBar(BuildContext context, {bool showBack = false, int? step, int total = 3, VoidCallback? onBack})`
  - `Widget obProgressDots(int current, int total)`
  - `Widget obHeading(BuildContext context, String text)`
  - `Widget obSubtitle(BuildContext context, String text)`
  - `Widget obPrimaryButton(String label, VoidCallback? onPressed)`
  - `Widget obSecondaryLink(String label, VoidCallback onPressed)`
  - `Widget obFeatureRow(BuildContext context, IconData icon, String title, String sub)`

- [ ] **Step 1: Create the file with moved helpers**

Copy the bodies of `_topBar` (`onboarding_flow.dart:494-522`), `_progressDots` (`524-541`), `_heading` (`543-549`), `_subtitle` (`551-559`), `_primaryButton` (`561-569`), `_secondaryLink` (`571-576`), `_featureRow` (`734-768`) into `mobile/lib/onboarding/stages/shared.dart` as top-level functions with the signatures above. Mechanical changes only:
- Add `import 'package:flutter/material.dart';`, `import '../../theme/colors.dart';`, `import '../../theme/typography.dart';` (match the imports the copied code references — check `onboarding_flow.dart` header for exact paths of `CwColors`, `CwTypography`).
- Prefix `Theme.of(context)` calls — `context` is now a parameter.
- In `obTopBar`, replace `onTap: _goBack` with `onTap: onBack`.
- Rename `_progressDots(...)` internal call in `obTopBar` to `obProgressDots(...)`.

- [ ] **Step 2: Analyze**

Run: `cd mobile && flutter analyze lib/onboarding/stages/shared.dart`
Expected: zero issues (file compiles standalone; unused-during-build is fine).

- [ ] **Step 3: Commit**

```bash
git add mobile/lib/onboarding/stages/shared.dart
git commit -m "refactor(onboarding): extract shared stage widgets (COW-26)"
```

---
### Task 5: Pre-DKG stages — hero, email, emailOtp

**Files:**
- Create: `mobile/lib/onboarding/stages/hero_stage.dart`
- Create: `mobile/lib/onboarding/stages/email_stage.dart`
- Create: `mobile/lib/onboarding/stages/otp_stage.dart`

Each stage is a `StatefulWidget` returning a `Scaffold(backgroundColor: CwColors.bgPaper, body: SafeArea(child: <moved markup>))`. It reads `final c = OnboardingScope.of(context);` in `build`. Local UI state (`TextEditingController`s, `_emailSending`, `_emailError`, `_otpVerifying`, `_otpError`, `PageController`, `_guidePage`) stays **local** to the stage. Shared state and side-effects go through `c`.

**hero_stage.dart** — move `_heroStage` (`580-651`), `_heroPage` (`653-709`), `_introPageContent` (`711-732`). Owns `_pageCtrl`/`_guidePage` locally. Rewrites:
- CTA "start" branch: `_goTo(_Stage.email)` → `c.goTo(OnboardingStep.email)`.
- Recover link: `Navigator.pushNamed(context, '/recovery')` → `Navigator.of(context, rootNavigator: true).pushNamed('/recovery')`.
- Use `obPrimaryButton`, `obFeatureRow`, `obHeading`, `obSubtitle` from `shared.dart`.

**email_stage.dart** — move `_emailStage` (`1164-1246`), `_submitEmail` (`772-817`), `_showRecoveryDialog` (`819-879`), `_showReRegisterConfirm` (`881-931`), `_verifyBackupShardForReRegister` (`933-947`), `_pickLocalBackupForReRegister` (`951-981`), `_continueReRegisterWithShard` (`985-1013`). Owns `_emailCtrl`, `_emailSending`, `_emailError` locally. Rewrites:
- `_emailCtrl.text.trim()` reads stay local; on success set `c.email = _emailCtrl.text.trim();` before navigating.
- `_goTo(_Stage.emailOtp)` → `c.goTo(OnboardingStep.emailOtp)`.
- Re-register success: set `c.forceRegister = true; c.backupShardHash = <hash>;` then `c.goTo(OnboardingStep.emailOtp)`.
- `Navigator.pushNamed(context, AppRouter.recovery, ...)` → root navigator variant.

**otp_stage.dart** — move `_emailOtpStage` (`1097-1162`), `_onOtpChanged` (`1017-1022`), `_verifyEmailOtp` (`1024-1079`), `_resendOtp` (`1081-1095`). Owns `_otpCtrl`, `_otpVerifying`, `_otpError` locally. Rewrites:
- Read `c.email` for `_emailCtrl.text.trim()` (which no longer exists here) — use `c.email` everywhere the OTP code sent email to the API.
- Read `c.forceRegister`, `c.backupShardHash`.
- On success `_goTo(_Stage.creating)` → `c.goTo(OnboardingStep.creating)`.
- `obTopBar(context, showBack: true, step: 0, onBack: c.goBack)`.

- [ ] **Step 1: Create the three files** per the specs above (markup copied from the listed line ranges; only the rewrites listed change).

- [ ] **Step 2: Analyze**

Run: `cd mobile && flutter analyze lib/onboarding/stages/`
Expected: zero issues.

- [ ] **Step 3: Commit**

```bash
git add mobile/lib/onboarding/stages/hero_stage.dart mobile/lib/onboarding/stages/email_stage.dart mobile/lib/onboarding/stages/otp_stage.dart
git commit -m "refactor(onboarding): extract hero/email/otp stages (COW-26)"
```

---
### Task 6: DKG-boundary stages — creating, backup (guarded)

**Files:**
- Create: `mobile/lib/onboarding/stages/creating_stage.dart`
- Create: `mobile/lib/onboarding/stages/backup_stage.dart`

Both wrap their `Scaffold` in `PopScope(canPop: false, child: ...)` — the DKG boundary. Neither shows a back affordance in `obTopBar` (`showBack: false`).

**creating_stage.dart** — move `_creatingStage` (`1250-1329`) and `_checkLine` (`1331-1360`) markup; move the `_startCreating` logic (`145-240`) into the stage's own `initState`/method. Local state: `_createProgress`, `_createChecksDone`, `_createTimer`, `_isResuming`, `_createError`. Rewrites:
- `initState`: call `_startCreating()` — its existing body already calls `sessionManager.canResume()` and `runDkgWithRecovery()`, which satisfies the "auto-resume on restore" requirement (spec §持久化, creating row) with no extra code, since the stage is entered both on fresh push and on restore.
- Use `c.sessionManager` instead of building a local `MpcSessionManager`.
- On DKG success (`maybeAdvance`'s advance branch): replace `CowalletApp.of(context).setWalletAddress(generatedAddress!)` + `_goTo(_Stage.backup)` with:
  ```dart
  try { CowalletApp.of(context).setWalletAddress(generatedAddress!); } catch (_) {}
  c.onDkgSuccess();
  ```
- Retry button `_startCreating` → local `_startCreating`.

**backup_stage.dart** — move `_backupStage` (`1485-1553`) + `_backupOptionCard` (`1555-1603`); move `_saveBackup` (`327-398`) and `_skipBackup` (`400-403`). Local state: `_backupSaving`, `_backupDone`. Rewrites:
- On backup success `_goTo(_Stage.bio)` → `c.backupDone = true; c.goToBioFromBackup();`
- `_skipBackup`: `c.backupSkipped = true; c.goToBioFromBackup();`
- Keep `showTopToast(context, ...)` calls (context available in stage).

- [ ] **Step 1: Create both files** per specs.

- [ ] **Step 2: Analyze**

Run: `cd mobile && flutter analyze lib/onboarding/stages/`
Expected: zero issues.

- [ ] **Step 3: Commit**

```bash
git add mobile/lib/onboarding/stages/creating_stage.dart mobile/lib/onboarding/stages/backup_stage.dart
git commit -m "refactor(onboarding): extract creating/backup stages with PopScope guard (COW-26)"
```

---
### Task 7: Post-DKG returnable group — bio, name, ready, persona

**Files:**
- Create: `mobile/lib/onboarding/stages/bio_stage.dart`
- Create: `mobile/lib/onboarding/stages/name_stage.dart`
- Create: `mobile/lib/onboarding/stages/ready_stage.dart`
- Create: `mobile/lib/onboarding/stages/persona_stage.dart`

These four form the swipe-back group. `bio` shows no back (it is the group floor); `name`/`ready`/`persona` show `obTopBar(..., showBack: true, onBack: c.goBack)`.

**bio_stage.dart** — move `_bioStage` (`1367-1412`); move `_startBioScan` (`248-315`). Local: `_bioAuthenticating`, `_bioDone`. Rewrite: on done `_goTo(_Stage.name)` → `c.goTo(OnboardingStep.name)`.

**name_stage.dart** — move `_nameStage` (`1416-1481`); move `_submitName` (`318-324`). Local: `_nameCtrl`. Rewrite: keep `CowalletApp.of(context).setUserName(name)`; `_goTo(_Stage.ready)` → `c.goTo(OnboardingStep.ready)`.

**ready_stage.dart** — move `_readyStage` (`1607-1671`) + `_numberedStep` (`1673-1716`). Rewrite: `_goTo(_Stage.persona)` → `c.goTo(OnboardingStep.persona)`. Reads `CowalletApp.of(context).userName` (unchanged).

**persona_stage.dart** — move `_personaStage` (`1720-1772`) + `_personaCard` (`1774-1855`); move `_pickPersona` (`407-411`), `_skipPersona` (`413`), and `_finish` (`416-434`). Local: `_selectedPersona`. Rewrites:
- `_pickPersona`: `CowalletApp.of(context).setPersona(id)` (keep), then call `_finish()`.
- `_finish`: replace body with — capture `final app = CowalletApp.of(context); app.completeOnboarding(); c.backupSkipped/backupDone` already on `c`; then `await c.finish(context, app.walletAddress);` (controller clears step, persists metadata, roots to `/`). Remove the local metadata-persist duplication (now in `controller.finish`).

- [ ] **Step 1: Create all four files** per specs.

- [ ] **Step 2: Analyze**

Run: `cd mobile && flutter analyze lib/onboarding/stages/`
Expected: zero issues.

- [ ] **Step 3: Commit**

```bash
git add mobile/lib/onboarding/stages/bio_stage.dart mobile/lib/onboarding/stages/name_stage.dart mobile/lib/onboarding/stages/ready_stage.dart mobile/lib/onboarding/stages/persona_stage.dart
git commit -m "refactor(onboarding): extract bio/name/ready/persona stages (COW-26)"
```

---
### Task 8: Swap OnboardingFlow to a sub-Navigator host

**Files:**
- Modify: `mobile/lib/onboarding/onboarding_flow.dart` (full rewrite of the file)

**Interfaces:**
- Consumes: `initialStackFor`, `OnboardingStep`, `OnboardingRoutes` (T1); `OnboardingController` (T2); `OnboardingScope` (T3); all 9 stage widgets (T4–T7).
- Produces: `OnboardingFlow` — resolves the saved step async, then hosts a child `Navigator` seeded via `onGenerateInitialRoutes`, wrapped in `OnboardingScope`. Route building centralizes the `CupertinoPageRoute` + per-stage widget mapping.

- [ ] **Step 1: Rewrite the file**

Replace the entire contents of `mobile/lib/onboarding/onboarding_flow.dart` with:

```dart
import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../utils/secure_storage.dart';
import 'controller.dart';
import 'routes.dart';
import 'scope.dart';
import 'stages/hero_stage.dart';
import 'stages/email_stage.dart';
import 'stages/otp_stage.dart';
import 'stages/creating_stage.dart';
import 'stages/backup_stage.dart';
import 'stages/bio_stage.dart';
import 'stages/name_stage.dart';
import 'stages/ready_stage.dart';
import 'stages/persona_stage.dart';

/// Hosts the onboarding stages as a child Navigator. Each stage is its own
/// route; the initial stack is rebuilt from the persisted step so a killed
/// app resumes where it left off.
class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({super.key});

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  final OnboardingController _controller = OnboardingController();
  List<OnboardingStep>? _initialStack; // null until the saved step resolves

  @override
  void initState() {
    super.initState();
    _resolveInitialStack();
  }

  Future<void> _resolveInitialStack() async {
    final saved = await SecureStorage.get(SecureStorage.keyOnboardingStep);
    if (!mounted) return;
    setState(() => _initialStack = initialStackFor(saved));
  }

  Widget _stageWidget(OnboardingStep step) {
    switch (step) {
      case OnboardingStep.hero:
        return const HeroStage();
      case OnboardingStep.email:
        return const EmailStage();
      case OnboardingStep.emailOtp:
        return const OtpStage();
      case OnboardingStep.creating:
        return const CreatingStage();
      case OnboardingStep.backup:
        return const BackupStage();
      case OnboardingStep.bio:
        return const BioStage();
      case OnboardingStep.name:
        return const NameStage();
      case OnboardingStep.ready:
        return const ReadyStage();
      case OnboardingStep.persona:
        return const PersonaStage();
    }
  }

  Route<dynamic> _routeFor(OnboardingStep step) => CupertinoPageRoute(
        settings: RouteSettings(name: step.name),
        builder: (_) => _stageWidget(step),
      );

  Route<dynamic> _onGenerateRoute(RouteSettings settings) {
    final step = stepFromName(settings.name) ?? OnboardingStep.hero;
    return _routeFor(step);
  }

  List<Route<dynamic>> _onGenerateInitialRoutes(NavigatorState _, String __) =>
      _initialStack!.map(_routeFor).toList();

  @override
  Widget build(BuildContext context) {
    if (_initialStack == null) {
      // Brief hold while the persisted step resolves; native splash still covers
      // cold start, so this is only a frame or two.
      return const Scaffold(backgroundColor: CwColors.bgPaper);
    }
    return OnboardingScope(
      controller: _controller,
      child: Navigator(
        key: _controller.navigatorKey,
        initialRoute: _initialStack!.last.name,
        onGenerateInitialRoutes: _onGenerateInitialRoutes,
        onGenerateRoute: _onGenerateRoute,
      ),
    );
  }
}
```

- [ ] **Step 2: Analyze the whole app**

Run: `cd mobile && flutter analyze`
Expected: zero issues. (If a stage still references a removed `_OnboardingFlowState` helper, fix the import to `stages/shared.dart`.)

- [ ] **Step 3: Full test run**

Run: `cd mobile && flutter test`
Expected: existing suites + Tasks 1/3 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add mobile/lib/onboarding/onboarding_flow.dart
git commit -m "refactor(onboarding): host stages in a child Navigator (COW-26)"
```

---
### Task 9: Required widget tests — stage nav + creating guard

**Files:**
- Create: `mobile/test/onboarding/flow_navigation_test.dart`
- Create: `mobile/test/onboarding/creating_guard_test.dart`

**Design note:** The real stage widgets depend on `Services.*` (unavailable in a `flutter test` host), but the *navigation machinery* under test is `OnboardingController`'s real methods + a child `Navigator` + `PopScope`. So both tests mount a small in-file host that wires the **real** controller and `OnboardingScope` to **fake stage** widgets (buttons that call controller methods). This exercises the true forward/back/guard behavior without Services. No separate fake-controller file is needed (the controller has no eager Services dependency).

- [ ] **Step 1: Write flow_navigation_test.dart**

```dart
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

Widget _fakeStage(OnboardingController c, OnboardingStep step) {
  switch (step) {
    case OnboardingStep.hero:
      return Scaffold(
        body: Center(
          child: TextButton(
            key: const Key('to-email'),
            onPressed: () => c.goTo(OnboardingStep.email),
            child: const Text('hero'),
          ),
        ),
      );
    case OnboardingStep.email:
      return Scaffold(
        appBar: AppBar(leading: BackButton(onPressed: c.goBack)),
        body: const Center(child: Text('email')),
      );
    default:
      return Scaffold(body: Center(child: Text(step.name)));
  }
}

Widget _host(OnboardingController c, List<OnboardingStep> stack) {
  Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
        settings: RouteSettings(name: s.name),
        builder: (_) => _fakeStage(c, s),
      );
  return MaterialApp(
    home: OnboardingScope(
      controller: c,
      child: Navigator(
        key: c.navigatorKey,
        initialRoute: stack.last.name,
        onGenerateInitialRoutes: (_, __) => stack.map(routeFor).toList(),
        onGenerateRoute: (s) => routeFor(stepFromName(s.name) ?? OnboardingStep.hero),
      ),
    ),
  );
}

void main() {
  testWidgets('forward push then back returns to previous stage', (tester) async {
    final c = OnboardingController();
    await tester.pumpWidget(_host(c, [OnboardingStep.hero]));
    expect(find.text('hero'), findsOneWidget);

    await tester.tap(find.byKey(const Key('to-email')));
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);
    expect(find.text('hero'), findsNothing);

    c.goBack();
    await tester.pumpAndSettle();
    expect(find.text('hero'), findsOneWidget);
  });
}
```

- [ ] **Step 2: Run — expect PASS**

Run: `cd mobile && flutter test test/onboarding/flow_navigation_test.dart`
Expected: PASS.

- [ ] **Step 3: Write creating_guard_test.dart**

```dart
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

void main() {
  testWidgets('creating route blocks system back (canPop:false)', (tester) async {
    final c = OnboardingController();
    Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
          settings: RouteSettings(name: s.name),
          builder: (_) => const PopScope(
            canPop: false,
            child: Scaffold(body: Center(child: Text('creating'))),
          ),
        );
    await tester.pumpWidget(MaterialApp(
      home: OnboardingScope(
        controller: c,
        child: Navigator(
          key: c.navigatorKey,
          initialRoute: OnboardingStep.creating.name,
          onGenerateInitialRoutes: (_, __) => [routeFor(OnboardingStep.creating)],
          onGenerateRoute: (_) => routeFor(OnboardingStep.creating),
        ),
      ),
    ));
    expect(find.text('creating'), findsOneWidget);

    // Simulate a system back gesture/button.
    final handled = await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    // Route did NOT leave — the guard consumed the pop.
    expect(find.text('creating'), findsOneWidget);
    expect(handled, isTrue);
  });
}
```

- [ ] **Step 4: Run — expect PASS**

Run: `cd mobile && flutter test test/onboarding/creating_guard_test.dart`
Expected: PASS (guard keeps the route on screen).

- [ ] **Step 5: Full suite + analyze, then commit**

```bash
cd mobile && flutter analyze && flutter test
git add mobile/test/onboarding/flow_navigation_test.dart mobile/test/onboarding/creating_guard_test.dart
git commit -m "test(onboarding): stage nav + creating-guard widget tests (COW-26)"
```

---

## Self-Review

**Spec coverage:**
- Nested sub-Navigator (spec §架构决策) → Task 8. ✅
- OnboardingController + Scope (§组件结构) → Tasks 2, 3. ✅
- Stack model / DKG hard boundary, PopScope on creating+backup (§导航栈模型) → Task 6 (guards), controller transitions Task 2. ✅
- Transitions: otp→creating push, dkg→pushAndRemoveUntil(backup), backup→bio pushReplacement, finish→root '/' (§关键转场) → `goTo`/`onDkgSuccess`/`goToBioFromBackup`/`finish` in Task 2; wired Tasks 5–7. ✅
- Persistence / restore table + creating auto-resume (§持久化) → `initialStackFor` Task 1, `_resolveInitialStack`+`onGenerateInitialRoutes` Task 8, creating `initState` runs `runDkgWithRecovery` Task 6. ✅
- Old `_restoreStep` stageOrder bug removed → old file fully replaced Task 8. ✅
- `/recovery` via root navigator (§两个细节) → Task 5 rewrites (hero + email). ✅
- keyOnboardingStep write-on-enter / delete-on-finish → `persistStep` in every `goTo`/`onDkgSuccess`/`goToBioFromBackup`; `clearStep` in `finish` (Task 2). ✅
- Tests: stage fwd/back + creating guard (§测试方案) → Task 9. ✅
- Route names in module, not app_router.dart (§偏离) → Task 1. ✅

**Placeholder scan:** New files (T1/2/3/8/9) have complete code. T4–T7 are mechanical moves with exact source line ranges + explicit call-site rewrites — the source file supplies the verbatim markup, which is the correct DRY approach for a move-and-rewire refactor.

**Type consistency:** `initialStackFor`/`stepFromName`/`OnboardingStep`/`OnboardingRoutes` used identically in T1/T8/T9. Controller methods `goTo`/`goBack`/`onDkgSuccess`/`goToBioFromBackup`/`finish`/`persistStep`/`clearStep` defined T2, consumed T5–T8. `OnboardingScope.of` defined T3, used T5–T7/T9. Stage class names (`HeroStage`…`PersonaStage`) defined T5–T7, consumed T8. Consistent. ✅
