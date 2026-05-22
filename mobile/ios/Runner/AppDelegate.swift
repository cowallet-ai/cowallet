import Flutter
import UIKit
import FirebaseCore
import FirebaseMessaging

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate, MessagingDelegate {
  override func application(

    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // Clear Keychain on fresh install (Keychain persists after app deletion)
    let hasLaunchedKey = "com.cowallet.hasLaunchedBefore"
    if !UserDefaults.standard.bool(forKey: hasLaunchedKey) {
      clearKeychainData()
      UserDefaults.standard.set(true, forKey: hasLaunchedKey)
    }

    // Initialize Firebase
    FirebaseApp.configure()

    // Set Firebase Messaging delegate
    Messaging.messaging().delegate = self

    // Register for remote notifications
    if #available(iOS 10.0, *) {
      UNUserNotificationCenter.current().delegate = self
    }

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  // MARK: - Firebase Messaging Delegate

  func messaging(_ messaging: Messaging, didReceiveRegistrationToken fcmToken: String?) {
    if let token = fcmToken {
      print("[FCM] Registration token: \(token)")
      // Token will be sent to backend from Flutter side
    }
  }

  // Handle remote notifications
  override func application(
    _ application: UIApplication,
    didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
  ) {
    Messaging.messaging().apnsToken = deviceToken
  }

  override func application(
    _ application: UIApplication,
    didFailToRegisterForRemoteNotificationsWithError error: Error
  ) {
    print("[FCM] Failed to register for remote notifications: \(error)")
  }

  // MARK: - Keychain Cleanup

  private func clearKeychainData() {
    let secClasses = [
      kSecClassGenericPassword,
      kSecClassKey,
    ]
    for secClass in secClasses {
      let query: [String: Any] = [kSecClass as String: secClass]
      SecItemDelete(query as CFDictionary)
    }
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    
    // Also register MPC handlers here
    MpcSecureEnclaveHandler.register(with: engineBridge.pluginRegistry.registrar(forPlugin: "MpcSecureEnclaveHandler")!)
    MpcSecureStorageHandler.register(with: engineBridge.pluginRegistry.registrar(forPlugin: "MpcSecureStorageHandler")!)
    CloudBackupHandler.register(with: engineBridge.pluginRegistry.registrar(forPlugin: "CloudBackupHandler")!)
  }
}
