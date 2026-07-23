import 'package:cowallet/services/settings_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('SettingsService language', () {
    test('is null when no language has been saved', () async {
      SharedPreferences.setMockInitialValues({});

      final settings = SettingsService();
      await settings.init();

      expect(settings.language, isNull);
    });

    test('loads a persisted language preference', () async {
      SharedPreferences.setMockInitialValues({'settings_language': 'en'});

      final settings = SettingsService();
      await settings.init();

      expect(settings.language, 'en');
    });

    test('persists an explicit language selection', () async {
      SharedPreferences.setMockInitialValues({});

      final settings = SettingsService();
      await settings.init();
      await settings.setLanguage('zh');

      final prefs = await SharedPreferences.getInstance();
      expect(settings.language, 'zh');
      expect(prefs.getString('settings_language'), 'zh');
    });
  });
}
