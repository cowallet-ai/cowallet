// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appName => 'CoWallet';

  @override
  String get tagline => 'the wallet that reads you back';

  @override
  String get tabHome => 'Home';

  @override
  String get tabWallet => 'Wallet';

  @override
  String get tabAsk => 'ASK';

  @override
  String get tabAgents => 'Agents';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDefi => 'DeFi';

  @override
  String get heroKicker => 'Digital wallet · speaks your language';

  @override
  String get heroH1a => 'A wallet that';

  @override
  String get heroH1b => 'actually';

  @override
  String get heroH1em => 'listens';

  @override
  String get heroExplain => 'Like hiring a butler for your money — say \"send \$100 to Sarah\" and it does it. Don\'t feel like talking? Buttons work too.';

  @override
  String get heroFeat1h => 'No crypto knowledge needed';

  @override
  String get heroFeat1s => 'Send, receive, and earn just by saying so';

  @override
  String get heroFeat2h => '100+ financial networks';

  @override
  String get heroFeat2s => 'Works worldwide';

  @override
  String get heroFeat3h => 'AI does the errands';

  @override
  String get heroFeat3s => 'Just say the word';

  @override
  String get getStarted => 'Get started';

  @override
  String get heroLegal => 'By continuing you agree to our Terms and Privacy Policy';

  @override
  String get introH1 => 'How your wallet protects you';

  @override
  String get introSub => 'CoWallet uses threshold signatures to split your key into three pieces.';

  @override
  String get introBullet1h => 'Key split into three';

  @override
  String get introBullet1s => 'One on your phone, one on server, one kept by you. The full key never exists anywhere.';

  @override
  String get introBullet2h => 'Two needed to transact';

  @override
  String get introBullet2s => 'No single party — including CoWallet — can move your money alone.';

  @override
  String get introBullet3h => 'No seed phrase';

  @override
  String get introBullet3s => 'No 12 words to write down. Lose your phone, your backup + server recovers everything.';

  @override
  String get introStart => 'Start creating';

  @override
  String get emailH1 => 'Recovery Email';

  @override
  String get emailSub => 'Used to verify your identity during wallet recovery. We won\'t send spam.';

  @override
  String get emailHint => 'This email is only used for wallet recovery verification';

  @override
  String get invalidEmail => 'Please enter a valid email address';

  @override
  String get emailSendFailed => 'Failed to send code, please try again';

  @override
  String get emailAlreadyRegistered => 'Email already registered';

  @override
  String get emailAlreadyRegisteredDesc => 'This email is linked to an existing wallet. Go to recovery?';

  @override
  String get goRecovery => 'Recover';

  @override
  String get reRegister => 'Re-register';

  @override
  String get reRegisterDesc => 'This will create a new wallet. Original assets can only be recovered via the recovery flow.';

  @override
  String get reRegisterConfirm => 'Confirm Re-register';

  @override
  String get otpH1 => 'Enter Verification Code';

  @override
  String otpSub(String email) {
    return 'Code sent to $email';
  }

  @override
  String get otpResend => 'Resend code';

  @override
  String get otpInvalid => 'Invalid or expired code';

  @override
  String get creatingH1 => 'Splitting your key into three pieces';

  @override
  String get creatingSub => 'Moving your money requires any 2 of 3 keys. Stored separately — lose one, the other two still work. The full key never exists in one place.';

  @override
  String get cl1 => '1st key: stored on this phone';

  @override
  String get cl2 => '2nd key: stored in server vault';

  @override
  String get cl3 => '3rd key: kept by you';

  @override
  String get createError => 'Wallet creation failed. Please try again.';

  @override
  String get retry => 'Retry';

  @override
  String get bioH1 => 'Enable biometric authentication';

  @override
  String get bioSub => 'Just like unlocking your phone. Protect your wallet with fingerprint or face. Biometric data never leaves this device.';

  @override
  String get bioActivate => 'Turn on biometrics';

  @override
  String get bioSkip => 'Use a passcode instead';

  @override
  String get bioVerifying => 'Verifying...';

  @override
  String get bioDone => 'Biometrics ready';

  @override
  String get pinH1 => 'Set wallet passcode';

  @override
  String get pinSub => '6-digit passcode, required for every transaction.';

  @override
  String get pinConfirmH1 => 'Confirm passcode';

  @override
  String get pinConfirmSub => 'Enter the same passcode again to confirm.';

  @override
  String get pinMismatch => 'Passcodes don\'t match. Try again.';

  @override
  String get pinDone => 'Passcode set';

  @override
  String get nameH1 => 'What should I call you?';

  @override
  String get nameSub => 'A nickname works. No real name needed.';

  @override
  String get namePlaceholder => 'e.g. Alice, Mike, or a nickname';

  @override
  String get nameTooShort => 'Name too short';

  @override
  String get nameTooLong => 'Name too long';

  @override
  String get settings => 'Settings';

  @override
  String get security => 'Security';

  @override
  String get keySecurity => 'Key Security';

  @override
  String get conversation => 'Conversation';

  @override
  String get general => 'General';

  @override
  String get biometricAuth => 'Biometric Authentication';

  @override
  String get biometricAuthReason => 'Authenticate to proceed';

  @override
  String get biometricNotAvailable => 'Not available on this device';

  @override
  String get biometricEnable => 'Enable';

  @override
  String get biometricDisable => 'Disable';

  @override
  String get emergencyFreeze => 'Emergency Freeze';

  @override
  String get emergencyFreezeSub => 'Temporarily freeze all transactions';

  @override
  String get emergencyFreezeConfirmTitle => 'Freeze Wallet?';

  @override
  String get emergencyFreezeConfirmBody => 'All transactions will be blocked until you unfreeze.';

  @override
  String get emergencyFreezeActivated => 'Wallet frozen';

  @override
  String get emergencyFreezeDeactivated => 'Wallet unfrozen';

  @override
  String get frozenBanner => 'Wallet is frozen. Tap Security to unfreeze.';

  @override
  String get emergencyContact => 'Emergency Contact';

  @override
  String get emergencyContactSub => 'Set up trusted contacts for recovery';

  @override
  String get riskGuard => 'Risk Guard';

  @override
  String get riskGuardSub => 'Custom security rules and limits';

  @override
  String get keysCheckup => 'Keys Check-up';

  @override
  String get keysCheckupSub => 'Check the health of your key shards';

  @override
  String get onPhone => 'On Phone';

  @override
  String get inCloud => 'In Cloud';

  @override
  String get recovery => 'Recovery';

  @override
  String get allSafe => 'All Safe';

  @override
  String get keyStatusError => 'Issues Found';

  @override
  String get keyStatusWarning => 'Check Soon';

  @override
  String get rotateKeyShares => 'Rotate Key Shares';

  @override
  String get presignatures => 'Presignatures';

  @override
  String get presignaturesSub => 'Offline signing for faster transactions';

  @override
  String get lastRotation => 'Last rotation';

  @override
  String get never => 'Never';

  @override
  String get today => 'Today';

  @override
  String get voiceInput => 'Voice Input';

  @override
  String get voiceInputSub => 'Use voice to interact with AI assistant';

  @override
  String get aiModel => 'AI Model';

  @override
  String get aiModelSub => 'Choose your preferred AI assistant';

  @override
  String get language => 'Language';

  @override
  String get weeklyReport => 'Weekly Report';

  @override
  String get weeklyReportSub => 'Get weekly portfolio summary';

  @override
  String get redoOnboarding => 'Redo Onboarding';

  @override
  String get redoOnboardingSub => 'Start over with wallet creation';

  @override
  String get on => 'ON';

  @override
  String get off => 'OFF';

  @override
  String get resetWalletTitle => 'Wallet Has Balance';

  @override
  String get resetWalletHasBalance => 'Please transfer your assets to another wallet before resetting.';

  @override
  String get resetWalletGoTransfer => 'Go Transfer';

  @override
  String get resetWalletConfirmTitle => 'Reset Wallet?';

  @override
  String get resetWalletConfirmBody => 'This will delete your wallet. You\'ll need to recover from your backup.';

  @override
  String get resetWalletConfirm => 'Reset';

  @override
  String get resetWalletChecking => 'Checking balance...';

  @override
  String get signoff1 => 'Built with ❤️ for the future of money';

  @override
  String signoff2(String version) {
    return 'v$version';
  }

  @override
  String get cancel => 'Cancel';

  @override
  String get confirm => 'Confirm';

  @override
  String get save => 'Save';

  @override
  String get delete => 'Delete';

  @override
  String get copy => 'Copy';

  @override
  String get copied => 'Copied';

  @override
  String get send => 'Send';

  @override
  String get receive => 'Receive';

  @override
  String get swap => 'Swap';

  @override
  String get more => 'More';

  @override
  String get loading => 'Loading...';

  @override
  String get error => 'Error';

  @override
  String get success => 'Success';

  @override
  String get failed => 'Failed';

  @override
  String get retryLater => 'Please try again later';

  @override
  String get wallet => 'Wallet';

  @override
  String get home => 'Home';

  @override
  String get balance => 'Balance';

  @override
  String get transactions => 'Transactions';

  @override
  String get contacts => 'Contacts';

  @override
  String get scan => 'Scan QR';

  @override
  String get help => 'Help';

  @override
  String get amount => 'Amount';

  @override
  String get to => 'To';

  @override
  String get from => 'From';

  @override
  String get gas => 'Gas Fee';

  @override
  String get total => 'Total';

  @override
  String get max => 'Max';

  @override
  String get memo => 'Memo';

  @override
  String get optional => 'Optional';

  @override
  String get sendConfirmTitle => 'Confirm Send';

  @override
  String sendConfirmBody(String amount, String symbol, String address) {
    return 'Send $amount $symbol to $address?';
  }

  @override
  String get transactionSent => 'Transaction sent';

  @override
  String get transactionFailed => 'Transaction failed';

  @override
  String get insufficientBalance => 'Insufficient balance';

  @override
  String get contactName => 'Contact Name';

  @override
  String get contactAddress => 'Wallet Address';

  @override
  String get addContact => 'Add Contact';

  @override
  String get editContact => 'Edit Contact';

  @override
  String get deleteContact => 'Delete Contact';

  @override
  String get noContacts => 'No contacts yet';

  @override
  String get addFirstContact => 'Add your first contact';

  @override
  String get chatPlaceholder => 'Ask me anything...';

  @override
  String get newChat => 'New Chat';

  @override
  String get chatHistory => 'Chat History';

  @override
  String get clearHistory => 'Clear History';

  @override
  String get clearHistoryConfirm => 'Clear all chat history?';

  @override
  String get yield => 'Yield';

  @override
  String get pools => 'Pools';

  @override
  String get apr => 'APR';

  @override
  String get deposit => 'Deposit';

  @override
  String get withdraw => 'Withdraw';

  @override
  String get yourPosition => 'Your Position';

  @override
  String get totalValue => 'Total Value';

  @override
  String get comingSoon => 'Coming soon';
}
