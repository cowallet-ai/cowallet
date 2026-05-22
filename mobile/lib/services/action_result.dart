class ActionResult {
  final bool success;
  final bool needsConfirm;
  final String message;
  final Map<String, String> data;

  const ActionResult({
    required this.success,
    required this.message,
    this.needsConfirm = false,
    this.data = const {},
  });

  const ActionResult.ok(this.message, {this.data = const {}})
      : success = true, needsConfirm = false;

  const ActionResult.fail(this.message, {this.data = const {}})
      : success = false, needsConfirm = false;

  const ActionResult.confirm(this.message, {this.data = const {}})
      : success = false, needsConfirm = true;
}
