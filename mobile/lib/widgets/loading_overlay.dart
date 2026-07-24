import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class LoadingOverlay {
  static OverlayEntry? _entry;

  static void show(BuildContext context, {String? message}) {
    dismiss();
    _entry = OverlayEntry(
      builder: (_) => _LoadingWidget(message: message),
    );
    Overlay.of(context).insert(_entry!);
  }

  static void dismiss() {
    _entry?.remove();
    _entry = null;
  }
}

class _LoadingWidget extends StatelessWidget {
  final String? message;
  const _LoadingWidget({this.message});

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.black.withValues(alpha: 0.35),
      child: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 22),
          decoration: BoxDecoration(
            color: CwColors.bgCard,
            borderRadius: BorderRadius.circular(16),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.1),
                blurRadius: 12,
              ),
            ],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              SizedBox(
                width: 28,
                height: 28,
                child: CircularProgressIndicator(
                  strokeWidth: 2.5,
                  color: CwColors.accent,
                ),
              ),
              if (message != null) ...[
                const SizedBox(height: 14),
                Text(
                  message!,
                  style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13, color: CwColors.ink2),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
