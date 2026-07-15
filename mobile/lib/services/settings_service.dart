import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

enum IntentMode { onEnter, whileTyping }

enum AiModel { bedrock, deepseek }

class SettingsService extends ChangeNotifier {
  static const _keyBiometric = 'settings_biometric_enabled';
  static const _keyVoiceInput = 'settings_voice_input_enabled';
  static const _keyIntentMode = 'settings_intent_mode';
  static const _keyLanguage = 'settings_language';
  static const _keyWeeklyReport = 'settings_weekly_report_enabled';
  static const _keyEmergencyFreeze = 'settings_emergency_freeze_active';
  static const _keyAiModel = 'settings_ai_model';
  // Whether the user has consented to sharing conversation context (message,
  // wallet address, portfolio, contacts, etc.) with the third-party AI
  // providers that power the assistant. Required before any AI request is made
  // (App Store privacy requirement: disclose + obtain consent in-app, not only
  // in the privacy policy).
  static const _keyAiConsent = 'settings_ai_data_consent';

  late SharedPreferences _prefs;

  bool _biometricEnabled = false;
  bool _voiceInputEnabled = true;
  IntentMode _intentMode = IntentMode.onEnter;
  String _language = 'zh';
  bool _weeklyReportEnabled = false;
  bool _emergencyFreezeActive = false;
  AiModel _aiModel = AiModel.bedrock;
  bool _aiConsentGranted = false;

  // Getters
  bool get biometricEnabled => _biometricEnabled;
  bool get voiceInputEnabled => _voiceInputEnabled;
  IntentMode get intentMode => _intentMode;
  String get language => _language;
  bool get weeklyReportEnabled => _weeklyReportEnabled;
  bool get emergencyFreezeActive => _emergencyFreezeActive;
  AiModel get aiModel => _aiModel;
  bool get aiConsentGranted => _aiConsentGranted;

  /// The model value string to send to the backend API.
  String get aiModelValue => _aiModel == AiModel.bedrock ? 'bedrock' : 'deepseek';

  /// Initialize the service by loading persisted settings.
  Future<void> init() async {
    _prefs = await SharedPreferences.getInstance();
    _biometricEnabled = _prefs.getBool(_keyBiometric) ?? false;
    _voiceInputEnabled = _prefs.getBool(_keyVoiceInput) ?? true;
    _intentMode = _prefs.getString(_keyIntentMode) == 'whileTyping'
        ? IntentMode.whileTyping
        : IntentMode.onEnter;
    _language = _prefs.getString(_keyLanguage) ?? 'zh';
    _weeklyReportEnabled = _prefs.getBool(_keyWeeklyReport) ?? false;
    _emergencyFreezeActive = _prefs.getBool(_keyEmergencyFreeze) ?? false;
    _aiModel = _prefs.getString(_keyAiModel) == 'deepseek'
        ? AiModel.deepseek
        : AiModel.bedrock;
    _aiConsentGranted = _prefs.getBool(_keyAiConsent) ?? false;
  }

  // Setters that persist and notify

  Future<void> setBiometricEnabled(bool value) async {
    if (_biometricEnabled == value) return;
    _biometricEnabled = value;
    await _prefs.setBool(_keyBiometric, value);
    notifyListeners();
  }

  Future<void> setVoiceInputEnabled(bool value) async {
    if (_voiceInputEnabled == value) return;
    _voiceInputEnabled = value;
    await _prefs.setBool(_keyVoiceInput, value);
    notifyListeners();
  }

  Future<void> setIntentMode(IntentMode mode) async {
    if (_intentMode == mode) return;
    _intentMode = mode;
    await _prefs.setString(_keyIntentMode, mode == IntentMode.whileTyping ? 'whileTyping' : 'onEnter');
    notifyListeners();
  }

  Future<void> setLanguage(String lang) async {
    if (_language == lang) return;
    _language = lang;
    await _prefs.setString(_keyLanguage, lang);
    notifyListeners();
  }

  Future<void> setWeeklyReportEnabled(bool value) async {
    if (_weeklyReportEnabled == value) return;
    _weeklyReportEnabled = value;
    await _prefs.setBool(_keyWeeklyReport, value);
    notifyListeners();
  }

  Future<void> setEmergencyFreezeActive(bool value) async {
    if (_emergencyFreezeActive == value) return;
    _emergencyFreezeActive = value;
    await _prefs.setBool(_keyEmergencyFreeze, value);
    notifyListeners();
  }

  Future<void> setAiModel(AiModel value) async {
    if (_aiModel == value) return;
    _aiModel = value;
    await _prefs.setString(_keyAiModel, value == AiModel.bedrock ? 'bedrock' : 'deepseek');
    notifyListeners();
  }

  Future<void> setAiConsentGranted(bool value) async {
    if (_aiConsentGranted == value) return;
    _aiConsentGranted = value;
    await _prefs.setBool(_keyAiConsent, value);
    notifyListeners();
  }
}
