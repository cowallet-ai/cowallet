import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale) : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate = _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates = <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('zh')
  ];

  /// No description provided for @appName.
  ///
  /// In en, this message translates to:
  /// **'CoWallet'**
  String get appName;

  /// No description provided for @tagline.
  ///
  /// In en, this message translates to:
  /// **'the wallet that reads you back'**
  String get tagline;

  /// No description provided for @tabHome.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get tabHome;

  /// No description provided for @tabWallet.
  ///
  /// In en, this message translates to:
  /// **'Wallet'**
  String get tabWallet;

  /// No description provided for @tabAsk.
  ///
  /// In en, this message translates to:
  /// **'ASK'**
  String get tabAsk;

  /// No description provided for @tabAgents.
  ///
  /// In en, this message translates to:
  /// **'Agents'**
  String get tabAgents;

  /// No description provided for @tabSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get tabSettings;

  /// No description provided for @tabDefi.
  ///
  /// In en, this message translates to:
  /// **'DeFi'**
  String get tabDefi;

  /// No description provided for @heroKicker.
  ///
  /// In en, this message translates to:
  /// **'Digital wallet · speaks your language'**
  String get heroKicker;

  /// No description provided for @heroH1a.
  ///
  /// In en, this message translates to:
  /// **'A wallet that'**
  String get heroH1a;

  /// No description provided for @heroH1b.
  ///
  /// In en, this message translates to:
  /// **'actually'**
  String get heroH1b;

  /// No description provided for @heroH1em.
  ///
  /// In en, this message translates to:
  /// **'listens'**
  String get heroH1em;

  /// No description provided for @heroExplain.
  ///
  /// In en, this message translates to:
  /// **'Like hiring a butler for your money — say \"send \$100 to Sarah\" and it does it. Don\'t feel like talking? Buttons work too.'**
  String get heroExplain;

  /// No description provided for @heroFeat1h.
  ///
  /// In en, this message translates to:
  /// **'No crypto knowledge needed'**
  String get heroFeat1h;

  /// No description provided for @heroFeat1s.
  ///
  /// In en, this message translates to:
  /// **'Send, receive, and earn just by saying so'**
  String get heroFeat1s;

  /// No description provided for @heroFeat2h.
  ///
  /// In en, this message translates to:
  /// **'100+ financial networks'**
  String get heroFeat2h;

  /// No description provided for @heroFeat2s.
  ///
  /// In en, this message translates to:
  /// **'Works worldwide'**
  String get heroFeat2s;

  /// No description provided for @heroFeat3h.
  ///
  /// In en, this message translates to:
  /// **'AI does the errands'**
  String get heroFeat3h;

  /// No description provided for @heroFeat3s.
  ///
  /// In en, this message translates to:
  /// **'Just say the word'**
  String get heroFeat3s;

  /// No description provided for @getStarted.
  ///
  /// In en, this message translates to:
  /// **'Get started'**
  String get getStarted;

  /// No description provided for @heroLegal.
  ///
  /// In en, this message translates to:
  /// **'By continuing you agree to our Terms and Privacy Policy'**
  String get heroLegal;

  /// No description provided for @introH1.
  ///
  /// In en, this message translates to:
  /// **'How your wallet protects you'**
  String get introH1;

  /// No description provided for @introSub.
  ///
  /// In en, this message translates to:
  /// **'CoWallet uses threshold signatures to split your key into three pieces.'**
  String get introSub;

  /// No description provided for @introBullet1h.
  ///
  /// In en, this message translates to:
  /// **'Key split into three'**
  String get introBullet1h;

  /// No description provided for @introBullet1s.
  ///
  /// In en, this message translates to:
  /// **'One on your phone, one on server, one kept by you. The full key never exists anywhere.'**
  String get introBullet1s;

  /// No description provided for @introBullet2h.
  ///
  /// In en, this message translates to:
  /// **'Two needed to transact'**
  String get introBullet2h;

  /// No description provided for @introBullet2s.
  ///
  /// In en, this message translates to:
  /// **'No single party — including CoWallet — can move your money alone.'**
  String get introBullet2s;

  /// No description provided for @introBullet3h.
  ///
  /// In en, this message translates to:
  /// **'No seed phrase'**
  String get introBullet3h;

  /// No description provided for @introBullet3s.
  ///
  /// In en, this message translates to:
  /// **'No 12 words to write down. Lose your phone, your backup + server recovers everything.'**
  String get introBullet3s;

  /// No description provided for @introStart.
  ///
  /// In en, this message translates to:
  /// **'Start creating'**
  String get introStart;

  /// No description provided for @emailH1.
  ///
  /// In en, this message translates to:
  /// **'Recovery Email'**
  String get emailH1;

  /// No description provided for @emailSub.
  ///
  /// In en, this message translates to:
  /// **'Used to verify your identity during wallet recovery. We won\'t send spam.'**
  String get emailSub;

  /// No description provided for @emailHint.
  ///
  /// In en, this message translates to:
  /// **'This email is only used for wallet recovery verification'**
  String get emailHint;

  /// No description provided for @invalidEmail.
  ///
  /// In en, this message translates to:
  /// **'Please enter a valid email address'**
  String get invalidEmail;

  /// No description provided for @emailSendFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to send code, please try again'**
  String get emailSendFailed;

  /// No description provided for @emailAlreadyRegistered.
  ///
  /// In en, this message translates to:
  /// **'Email already registered'**
  String get emailAlreadyRegistered;

  /// No description provided for @emailAlreadyRegisteredDesc.
  ///
  /// In en, this message translates to:
  /// **'This email is linked to an existing wallet. Go to recovery?'**
  String get emailAlreadyRegisteredDesc;

  /// No description provided for @goRecovery.
  ///
  /// In en, this message translates to:
  /// **'Recover'**
  String get goRecovery;

  /// No description provided for @reRegister.
  ///
  /// In en, this message translates to:
  /// **'Re-register'**
  String get reRegister;

  /// No description provided for @reRegisterDesc.
  ///
  /// In en, this message translates to:
  /// **'This will create a new wallet. Original assets can only be recovered via the recovery flow.'**
  String get reRegisterDesc;

  /// No description provided for @reRegisterConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm Re-register'**
  String get reRegisterConfirm;

  /// No description provided for @otpH1.
  ///
  /// In en, this message translates to:
  /// **'Enter Verification Code'**
  String get otpH1;

  /// No description provided for @otpSub.
  ///
  /// In en, this message translates to:
  /// **'Code sent to {email}'**
  String otpSub(String email);

  /// No description provided for @otpResend.
  ///
  /// In en, this message translates to:
  /// **'Resend code'**
  String get otpResend;

  /// No description provided for @otpInvalid.
  ///
  /// In en, this message translates to:
  /// **'Invalid or expired code'**
  String get otpInvalid;

  /// No description provided for @creatingH1.
  ///
  /// In en, this message translates to:
  /// **'Splitting your key into three pieces'**
  String get creatingH1;

  /// No description provided for @creatingSub.
  ///
  /// In en, this message translates to:
  /// **'Moving your money requires any 2 of 3 keys. Stored separately — lose one, the other two still work. The full key never exists in one place.'**
  String get creatingSub;

  /// No description provided for @cl1.
  ///
  /// In en, this message translates to:
  /// **'1st key: stored on this phone'**
  String get cl1;

  /// No description provided for @cl2.
  ///
  /// In en, this message translates to:
  /// **'2nd key: stored in server vault'**
  String get cl2;

  /// No description provided for @cl3.
  ///
  /// In en, this message translates to:
  /// **'3rd key: kept by you'**
  String get cl3;

  /// No description provided for @createError.
  ///
  /// In en, this message translates to:
  /// **'Wallet creation failed. Please try again.'**
  String get createError;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @bioH1.
  ///
  /// In en, this message translates to:
  /// **'Enable biometric authentication'**
  String get bioH1;

  /// No description provided for @bioSub.
  ///
  /// In en, this message translates to:
  /// **'Just like unlocking your phone. Protect your wallet with fingerprint or face. Biometric data never leaves this device.'**
  String get bioSub;

  /// No description provided for @bioActivate.
  ///
  /// In en, this message translates to:
  /// **'Turn on biometrics'**
  String get bioActivate;

  /// No description provided for @bioSkip.
  ///
  /// In en, this message translates to:
  /// **'Use a passcode instead'**
  String get bioSkip;

  /// No description provided for @bioVerifying.
  ///
  /// In en, this message translates to:
  /// **'Verifying...'**
  String get bioVerifying;

  /// No description provided for @bioDone.
  ///
  /// In en, this message translates to:
  /// **'Biometrics ready'**
  String get bioDone;

  /// No description provided for @pinH1.
  ///
  /// In en, this message translates to:
  /// **'Set wallet passcode'**
  String get pinH1;

  /// No description provided for @pinSub.
  ///
  /// In en, this message translates to:
  /// **'6-digit passcode, required for every transaction.'**
  String get pinSub;

  /// No description provided for @pinConfirmH1.
  ///
  /// In en, this message translates to:
  /// **'Confirm passcode'**
  String get pinConfirmH1;

  /// No description provided for @pinConfirmSub.
  ///
  /// In en, this message translates to:
  /// **'Enter the same passcode again to confirm.'**
  String get pinConfirmSub;

  /// No description provided for @pinMismatch.
  ///
  /// In en, this message translates to:
  /// **'Passcodes don\'t match. Try again.'**
  String get pinMismatch;

  /// No description provided for @pinDone.
  ///
  /// In en, this message translates to:
  /// **'Passcode set'**
  String get pinDone;

  /// No description provided for @nameH1.
  ///
  /// In en, this message translates to:
  /// **'What should I call you?'**
  String get nameH1;

  /// No description provided for @nameSub.
  ///
  /// In en, this message translates to:
  /// **'A nickname works. No real name needed.'**
  String get nameSub;

  /// No description provided for @namePlaceholder.
  ///
  /// In en, this message translates to:
  /// **'e.g. Alice, Mike, or a nickname'**
  String get namePlaceholder;

  /// No description provided for @nameTooShort.
  ///
  /// In en, this message translates to:
  /// **'Name too short'**
  String get nameTooShort;

  /// No description provided for @nameTooLong.
  ///
  /// In en, this message translates to:
  /// **'Name too long'**
  String get nameTooLong;

  /// No description provided for @settings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settings;

  /// No description provided for @security.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get security;

  /// No description provided for @keySecurity.
  ///
  /// In en, this message translates to:
  /// **'Key Security'**
  String get keySecurity;

  /// No description provided for @conversation.
  ///
  /// In en, this message translates to:
  /// **'Conversation'**
  String get conversation;

  /// No description provided for @general.
  ///
  /// In en, this message translates to:
  /// **'General'**
  String get general;

  /// No description provided for @biometricAuth.
  ///
  /// In en, this message translates to:
  /// **'Biometric Authentication'**
  String get biometricAuth;

  /// No description provided for @biometricAuthReason.
  ///
  /// In en, this message translates to:
  /// **'Authenticate to proceed'**
  String get biometricAuthReason;

  /// No description provided for @biometricNotAvailable.
  ///
  /// In en, this message translates to:
  /// **'Not available on this device'**
  String get biometricNotAvailable;

  /// No description provided for @biometricEnable.
  ///
  /// In en, this message translates to:
  /// **'Enable'**
  String get biometricEnable;

  /// No description provided for @biometricDisable.
  ///
  /// In en, this message translates to:
  /// **'Disable'**
  String get biometricDisable;

  /// No description provided for @emergencyFreeze.
  ///
  /// In en, this message translates to:
  /// **'Emergency Freeze'**
  String get emergencyFreeze;

  /// No description provided for @emergencyFreezeSub.
  ///
  /// In en, this message translates to:
  /// **'Temporarily freeze all transactions'**
  String get emergencyFreezeSub;

  /// No description provided for @emergencyFreezeConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Freeze Wallet?'**
  String get emergencyFreezeConfirmTitle;

  /// No description provided for @emergencyFreezeConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'All transactions will be blocked until you unfreeze.'**
  String get emergencyFreezeConfirmBody;

  /// No description provided for @emergencyFreezeActivated.
  ///
  /// In en, this message translates to:
  /// **'Wallet frozen'**
  String get emergencyFreezeActivated;

  /// No description provided for @emergencyFreezeDeactivated.
  ///
  /// In en, this message translates to:
  /// **'Wallet unfrozen'**
  String get emergencyFreezeDeactivated;

  /// No description provided for @frozenBanner.
  ///
  /// In en, this message translates to:
  /// **'Wallet is frozen. Tap Security to unfreeze.'**
  String get frozenBanner;

  /// No description provided for @emergencyContact.
  ///
  /// In en, this message translates to:
  /// **'Emergency Contact'**
  String get emergencyContact;

  /// No description provided for @emergencyContactSub.
  ///
  /// In en, this message translates to:
  /// **'Set up trusted contacts for recovery'**
  String get emergencyContactSub;

  /// No description provided for @riskGuard.
  ///
  /// In en, this message translates to:
  /// **'Risk Guard'**
  String get riskGuard;

  /// No description provided for @riskGuardSub.
  ///
  /// In en, this message translates to:
  /// **'Custom security rules and limits'**
  String get riskGuardSub;

  /// No description provided for @keysCheckup.
  ///
  /// In en, this message translates to:
  /// **'Keys Check-up'**
  String get keysCheckup;

  /// No description provided for @keysCheckupSub.
  ///
  /// In en, this message translates to:
  /// **'Check the health of your key shards'**
  String get keysCheckupSub;

  /// No description provided for @onPhone.
  ///
  /// In en, this message translates to:
  /// **'On Phone'**
  String get onPhone;

  /// No description provided for @inCloud.
  ///
  /// In en, this message translates to:
  /// **'In Cloud'**
  String get inCloud;

  /// No description provided for @recovery.
  ///
  /// In en, this message translates to:
  /// **'Recovery'**
  String get recovery;

  /// No description provided for @allSafe.
  ///
  /// In en, this message translates to:
  /// **'All Safe'**
  String get allSafe;

  /// No description provided for @keyStatusError.
  ///
  /// In en, this message translates to:
  /// **'Issues Found'**
  String get keyStatusError;

  /// No description provided for @keyStatusWarning.
  ///
  /// In en, this message translates to:
  /// **'Check Soon'**
  String get keyStatusWarning;

  /// No description provided for @rotateKeyShares.
  ///
  /// In en, this message translates to:
  /// **'Rotate Key Shares'**
  String get rotateKeyShares;

  /// No description provided for @presignatures.
  ///
  /// In en, this message translates to:
  /// **'Presignatures'**
  String get presignatures;

  /// No description provided for @presignaturesSub.
  ///
  /// In en, this message translates to:
  /// **'Offline signing for faster transactions'**
  String get presignaturesSub;

  /// No description provided for @lastRotation.
  ///
  /// In en, this message translates to:
  /// **'Last rotation'**
  String get lastRotation;

  /// No description provided for @never.
  ///
  /// In en, this message translates to:
  /// **'Never'**
  String get never;

  /// No description provided for @today.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get today;

  /// No description provided for @voiceInput.
  ///
  /// In en, this message translates to:
  /// **'Voice Input'**
  String get voiceInput;

  /// No description provided for @voiceInputSub.
  ///
  /// In en, this message translates to:
  /// **'Use voice to interact with AI assistant'**
  String get voiceInputSub;

  /// No description provided for @aiModel.
  ///
  /// In en, this message translates to:
  /// **'AI Model'**
  String get aiModel;

  /// No description provided for @aiModelSub.
  ///
  /// In en, this message translates to:
  /// **'Choose your preferred AI assistant'**
  String get aiModelSub;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @weeklyReport.
  ///
  /// In en, this message translates to:
  /// **'Weekly Report'**
  String get weeklyReport;

  /// No description provided for @weeklyReportSub.
  ///
  /// In en, this message translates to:
  /// **'Get weekly portfolio summary'**
  String get weeklyReportSub;

  /// No description provided for @redoOnboarding.
  ///
  /// In en, this message translates to:
  /// **'Redo Onboarding'**
  String get redoOnboarding;

  /// No description provided for @redoOnboardingSub.
  ///
  /// In en, this message translates to:
  /// **'Start over with wallet creation'**
  String get redoOnboardingSub;

  /// No description provided for @on.
  ///
  /// In en, this message translates to:
  /// **'ON'**
  String get on;

  /// No description provided for @off.
  ///
  /// In en, this message translates to:
  /// **'OFF'**
  String get off;

  /// No description provided for @resetWalletTitle.
  ///
  /// In en, this message translates to:
  /// **'Wallet Has Balance'**
  String get resetWalletTitle;

  /// No description provided for @resetWalletHasBalance.
  ///
  /// In en, this message translates to:
  /// **'Please transfer your assets to another wallet before resetting.'**
  String get resetWalletHasBalance;

  /// No description provided for @resetWalletGoTransfer.
  ///
  /// In en, this message translates to:
  /// **'Go Transfer'**
  String get resetWalletGoTransfer;

  /// No description provided for @resetWalletConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Reset Wallet?'**
  String get resetWalletConfirmTitle;

  /// No description provided for @resetWalletConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'This will delete your wallet. You\'ll need to recover from your backup.'**
  String get resetWalletConfirmBody;

  /// No description provided for @resetWalletConfirm.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get resetWalletConfirm;

  /// No description provided for @resetWalletChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking balance...'**
  String get resetWalletChecking;

  /// No description provided for @signoff1.
  ///
  /// In en, this message translates to:
  /// **'CoWallet · 2026'**
  String get signoff1;

  /// No description provided for @signoff2.
  ///
  /// In en, this message translates to:
  /// **'v{version}'**
  String signoff2(String version);

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @confirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirm;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @copy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get copy;

  /// No description provided for @copied.
  ///
  /// In en, this message translates to:
  /// **'Copied'**
  String get copied;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @receive.
  ///
  /// In en, this message translates to:
  /// **'Receive'**
  String get receive;

  /// No description provided for @swap.
  ///
  /// In en, this message translates to:
  /// **'Swap'**
  String get swap;

  /// No description provided for @more.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get more;

  /// No description provided for @loading.
  ///
  /// In en, this message translates to:
  /// **'Loading...'**
  String get loading;

  /// No description provided for @error.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get error;

  /// No description provided for @success.
  ///
  /// In en, this message translates to:
  /// **'Success'**
  String get success;

  /// No description provided for @failed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get failed;

  /// No description provided for @retryLater.
  ///
  /// In en, this message translates to:
  /// **'Please try again later'**
  String get retryLater;

  /// No description provided for @wallet.
  ///
  /// In en, this message translates to:
  /// **'Wallet'**
  String get wallet;

  /// No description provided for @home.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get home;

  /// No description provided for @balance.
  ///
  /// In en, this message translates to:
  /// **'Balance'**
  String get balance;

  /// No description provided for @transactions.
  ///
  /// In en, this message translates to:
  /// **'Transactions'**
  String get transactions;

  /// No description provided for @contacts.
  ///
  /// In en, this message translates to:
  /// **'Contacts'**
  String get contacts;

  /// No description provided for @scan.
  ///
  /// In en, this message translates to:
  /// **'Scan QR'**
  String get scan;

  /// No description provided for @help.
  ///
  /// In en, this message translates to:
  /// **'Help'**
  String get help;

  /// No description provided for @amount.
  ///
  /// In en, this message translates to:
  /// **'Amount'**
  String get amount;

  /// No description provided for @to.
  ///
  /// In en, this message translates to:
  /// **'To'**
  String get to;

  /// No description provided for @from.
  ///
  /// In en, this message translates to:
  /// **'From'**
  String get from;

  /// No description provided for @gas.
  ///
  /// In en, this message translates to:
  /// **'Gas Fee'**
  String get gas;

  /// No description provided for @total.
  ///
  /// In en, this message translates to:
  /// **'Total'**
  String get total;

  /// No description provided for @max.
  ///
  /// In en, this message translates to:
  /// **'Max'**
  String get max;

  /// No description provided for @memo.
  ///
  /// In en, this message translates to:
  /// **'Memo'**
  String get memo;

  /// No description provided for @optional.
  ///
  /// In en, this message translates to:
  /// **'Optional'**
  String get optional;

  /// No description provided for @sendConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Confirm Send'**
  String get sendConfirmTitle;

  /// No description provided for @sendConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Send {amount} {symbol} to {address}?'**
  String sendConfirmBody(String amount, String symbol, String address);

  /// No description provided for @transactionSent.
  ///
  /// In en, this message translates to:
  /// **'Transaction sent'**
  String get transactionSent;

  /// No description provided for @transactionFailed.
  ///
  /// In en, this message translates to:
  /// **'Transaction failed'**
  String get transactionFailed;

  /// No description provided for @insufficientBalance.
  ///
  /// In en, this message translates to:
  /// **'Insufficient balance'**
  String get insufficientBalance;

  /// No description provided for @contactName.
  ///
  /// In en, this message translates to:
  /// **'Contact Name'**
  String get contactName;

  /// No description provided for @contactAddress.
  ///
  /// In en, this message translates to:
  /// **'Wallet Address'**
  String get contactAddress;

  /// No description provided for @addContact.
  ///
  /// In en, this message translates to:
  /// **'Add Contact'**
  String get addContact;

  /// No description provided for @editContact.
  ///
  /// In en, this message translates to:
  /// **'Edit Contact'**
  String get editContact;

  /// No description provided for @deleteContact.
  ///
  /// In en, this message translates to:
  /// **'Delete Contact'**
  String get deleteContact;

  /// No description provided for @noContacts.
  ///
  /// In en, this message translates to:
  /// **'No contacts yet'**
  String get noContacts;

  /// No description provided for @addFirstContact.
  ///
  /// In en, this message translates to:
  /// **'Add your first contact'**
  String get addFirstContact;

  /// No description provided for @chatPlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Ask me anything...'**
  String get chatPlaceholder;

  /// No description provided for @newChat.
  ///
  /// In en, this message translates to:
  /// **'New Chat'**
  String get newChat;

  /// No description provided for @chatHistory.
  ///
  /// In en, this message translates to:
  /// **'Chat History'**
  String get chatHistory;

  /// No description provided for @clearHistory.
  ///
  /// In en, this message translates to:
  /// **'Clear History'**
  String get clearHistory;

  /// No description provided for @clearHistoryConfirm.
  ///
  /// In en, this message translates to:
  /// **'Clear all chat history?'**
  String get clearHistoryConfirm;

  /// No description provided for @yield.
  ///
  /// In en, this message translates to:
  /// **'Yield'**
  String get yield;

  /// No description provided for @pools.
  ///
  /// In en, this message translates to:
  /// **'Pools'**
  String get pools;

  /// No description provided for @apr.
  ///
  /// In en, this message translates to:
  /// **'APR'**
  String get apr;

  /// No description provided for @deposit.
  ///
  /// In en, this message translates to:
  /// **'Deposit'**
  String get deposit;

  /// No description provided for @withdraw.
  ///
  /// In en, this message translates to:
  /// **'Withdraw'**
  String get withdraw;

  /// No description provided for @yourPosition.
  ///
  /// In en, this message translates to:
  /// **'Your Position'**
  String get yourPosition;

  /// No description provided for @totalValue.
  ///
  /// In en, this message translates to:
  /// **'Total Value'**
  String get totalValue;

  /// No description provided for @comingSoon.
  ///
  /// In en, this message translates to:
  /// **'Coming soon'**
  String get comingSoon;

  /// No description provided for @quickSetup.
  ///
  /// In en, this message translates to:
  /// **'Quick Setup'**
  String get quickSetup;

  /// No description provided for @dailyLimit.
  ///
  /// In en, this message translates to:
  /// **'Daily Limit'**
  String get dailyLimit;

  /// No description provided for @dailyLimitDesc.
  ///
  /// In en, this message translates to:
  /// **'Daily transfer limit up to \${amount}'**
  String dailyLimitDesc(String amount);

  /// No description provided for @largeTransferConfirm.
  ///
  /// In en, this message translates to:
  /// **'Large Transfer Confirm'**
  String get largeTransferConfirm;

  /// No description provided for @largeTransferDesc.
  ///
  /// In en, this message translates to:
  /// **'Single transfer over \${amount} requires confirmation'**
  String largeTransferDesc(String amount);

  /// No description provided for @activePolicies.
  ///
  /// In en, this message translates to:
  /// **'Active Policies'**
  String get activePolicies;

  /// No description provided for @noPolicies.
  ///
  /// In en, this message translates to:
  /// **'No policies. Use quick setup above.'**
  String get noPolicies;

  /// No description provided for @deletePolicyConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete \"\${name}\"?'**
  String deletePolicyConfirm(String name);

  /// No description provided for @yesterday.
  ///
  /// In en, this message translates to:
  /// **'Yesterday'**
  String get yesterday;

  /// No description provided for @daysAgo.
  ///
  /// In en, this message translates to:
  /// **'\${days} days ago'**
  String daysAgo(int days);

  /// No description provided for @monthsAgo.
  ///
  /// In en, this message translates to:
  /// **'\${months} months ago'**
  String monthsAgo(int months);

  /// No description provided for @languageLabel.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get languageLabel;

  /// No description provided for @contactSaved.
  ///
  /// In en, this message translates to:
  /// **'✅ Contact \"\${name}\" saved'**
  String contactSaved(String name);

  /// No description provided for @releaseToCancel.
  ///
  /// In en, this message translates to:
  /// **'Release to cancel'**
  String get releaseToCancel;

  /// No description provided for @slideUpToSend.
  ///
  /// In en, this message translates to:
  /// **'Slide up to cancel, release to send'**
  String get slideUpToSend;

  /// No description provided for @saveContact.
  ///
  /// In en, this message translates to:
  /// **'Save Contact'**
  String get saveContact;

  /// No description provided for @cautions.
  ///
  /// In en, this message translates to:
  /// **'Cautions'**
  String get cautions;

  /// No description provided for @passed.
  ///
  /// In en, this message translates to:
  /// **'Passed'**
  String get passed;

  /// No description provided for @safetyAdvice.
  ///
  /// In en, this message translates to:
  /// **'Safety Advice'**
  String get safetyAdvice;

  /// No description provided for @riskLevelSafe.
  ///
  /// In en, this message translates to:
  /// **'Safe'**
  String get riskLevelSafe;

  /// No description provided for @riskLevelMedium.
  ///
  /// In en, this message translates to:
  /// **'Medium Risk'**
  String get riskLevelMedium;

  /// No description provided for @riskLevelHigh.
  ///
  /// In en, this message translates to:
  /// **'High Risk'**
  String get riskLevelHigh;

  /// No description provided for @riskLevelUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get riskLevelUnknown;

  /// No description provided for @securityAudit.
  ///
  /// In en, this message translates to:
  /// **'Security audit'**
  String get securityAudit;

  /// No description provided for @riskItems.
  ///
  /// In en, this message translates to:
  /// **'Risk Items'**
  String get riskItems;

  /// No description provided for @allEvmChains.
  ///
  /// In en, this message translates to:
  /// **'All EVM chains supported'**
  String get allEvmChains;

  /// No description provided for @addressCopied.
  ///
  /// In en, this message translates to:
  /// **'Address copied'**
  String get addressCopied;

  /// No description provided for @copyAddress.
  ///
  /// In en, this message translates to:
  /// **'Copy Address'**
  String get copyAddress;

  /// No description provided for @transferRejected.
  ///
  /// In en, this message translates to:
  /// **'Transfer Rejected'**
  String get transferRejected;

  /// No description provided for @transferExceedsLimit.
  ///
  /// In en, this message translates to:
  /// **'Exceeds Transfer Limit'**
  String get transferExceedsLimit;

  /// No description provided for @limitLabel.
  ///
  /// In en, this message translates to:
  /// **'Limit: \${limit}'**
  String limitLabel(String limit);

  /// No description provided for @adjustLimitHint.
  ///
  /// In en, this message translates to:
  /// **'Adjust the amount or modify your limit in Settings > Transfer Limits.'**
  String get adjustLimitHint;

  /// No description provided for @featureAiIntent.
  ///
  /// In en, this message translates to:
  /// **'Tell me about AI intent recognition'**
  String get featureAiIntent;

  /// No description provided for @featureProxyPay.
  ///
  /// In en, this message translates to:
  /// **'Tell me about proxy payment'**
  String get featureProxyPay;

  /// No description provided for @featureFamily.
  ///
  /// In en, this message translates to:
  /// **'Tell me about family shared wallet'**
  String get featureFamily;

  /// No description provided for @featureSkills.
  ///
  /// In en, this message translates to:
  /// **'Tell me about skill extensions'**
  String get featureSkills;

  /// No description provided for @checkBalance.
  ///
  /// In en, this message translates to:
  /// **'Check balance'**
  String get checkBalance;

  /// No description provided for @chainBalance.
  ///
  /// In en, this message translates to:
  /// **'Check \${token} balance on \${chain}'**
  String chainBalance(String chain, String token);

  /// No description provided for @recentTransactions.
  ///
  /// In en, this message translates to:
  /// **'Recent transactions'**
  String get recentTransactions;

  /// No description provided for @backupVerifyFailed.
  ///
  /// In en, this message translates to:
  /// **'Backup verification failed. Please confirm you imported the correct backup file from registration.'**
  String get backupVerifyFailed;

  /// No description provided for @backupFormatInvalid.
  ///
  /// In en, this message translates to:
  /// **'Backup format is invalid. Please check if the file is complete.'**
  String get backupFormatInvalid;

  /// No description provided for @noCloudBackup.
  ///
  /// In en, this message translates to:
  /// **'No cloud backup found. Please try importing from a file.'**
  String get noCloudBackup;

  /// No description provided for @viewChainAssets.
  ///
  /// In en, this message translates to:
  /// **'View assets on \${chain}'**
  String viewChainAssets(String chain);

  /// No description provided for @aaveUsdc.
  ///
  /// In en, this message translates to:
  /// **'USDC on Aave'**
  String get aaveUsdc;

  /// No description provided for @baseAudited.
  ///
  /// In en, this message translates to:
  /// **'Base Chain · Audited'**
  String get baseAudited;

  /// No description provided for @agentRule1.
  ///
  /// In en, this message translates to:
  /// **'Read-only balance, \$500 daily limit, no staking'**
  String get agentRule1;

  /// No description provided for @agentSigned.
  ///
  /// In en, this message translates to:
  /// **'Signed \${signed}/\${total}'**
  String agentSigned(int signed, int total);

  /// No description provided for @agentRule2.
  ///
  /// In en, this message translates to:
  /// **'Team expense, auto-pay after approval'**
  String get agentRule2;

  /// No description provided for @pinError.
  ///
  /// In en, this message translates to:
  /// **'Incorrect PIN. \${attempts} attempts remaining'**
  String pinError(int attempts);
}

class _AppLocalizationsDelegate extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => true;
}

AppLocalizations lookupAppLocalizations(Locale locale) {


  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en': return AppLocalizationsEn();
    case 'zh': return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.'
  );
}
