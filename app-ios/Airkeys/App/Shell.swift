import AirkeysCore
import CoreBluetooth
import CoreTelephony
import Network
import Observation
import UIKit

/// The iOS side of the core: performs what it asks (camera, screen), reports the radios, and
/// holds the newest view for SwiftUI. Which screen, what it says and what may be signed are
/// the core's decisions. Nothing is stored: the phrase lives in the core's memory, for as long
/// as the app runs.
@Observable
final class Shell {
    private let core = AirkeysCore()
    private(set) var view: ViewModel
    /// The core wants the camera on (the scan screen shows the preview then).
    private(set) var cameraOn = false
    /// A recovery phrase, a passphrase or the SeedQR camera is on screen.
    private(set) var privateScreen = false
    @ObservationIgnored private var brightness: CGFloat?
    /// The core wants the screen bright (a code is shown); iOS keeps an app's brightness after
    /// it leaves, so it is given back whenever the app is not in front.
    @ObservationIgnored private var bright = false
    @ObservationIgnored private var started = false
    @ObservationIgnored private lazy var radios = Radios { [weak self] wifi, bluetooth, cellular, network in
        self?.dispatch(.radios(wifi: wifi, bluetooth: bluetooth, cellular: cellular, network: network))
    }
    @ObservationIgnored lazy var camera = Camera(
        onFrame: { [weak self] frame in self?.frame(frame) },
        onError: { [weak self] message in DispatchQueue.main.async { self?.dispatch(.cameraFailed(message: message)) } }
    )
    /// Debug builds: `AIRKEYS_DELIVER=<base64 message>` hands the scan screen a message as if
    /// the camera had read it (the simulator has no camera). Same review, same confirmation.
    @ObservationIgnored private var pendingDelivery: Data?

    init() {
        view = core.view()
    }

    func start() {
        guard !started else { return }
        started = true
        #if DEBUG
        pendingDelivery = ProcessInfo.processInfo.environment["AIRKEYS_DELIVER"].flatMap { Data(base64Encoded: $0) }
        #endif
        radios.start()
    }

    func dispatch(_ action: Action) {
        apply(core.dispatch(action: action))
    }

    /// An edit of the phrase field; returns the core's text for it (words completed).
    func phraseInput(_ text: String) -> String {
        dispatch(.phraseInput(text: text))
        if case let .import(phrase) = view.screen { return phrase.text }
        return text
    }

    func codeFrame(_ width: UInt32, _ height: UInt32) -> CodeFrame? { core.codeFrame(width: width, height: height) }

    private func frame(_ frame: CameraFrame) {
        guard let update = core.scan(frame: frame) else { return }
        DispatchQueue.main.async { self.apply(update) }
    }

    private func apply(_ update: Update) {
        if update.view.rev >= view.rev { view = update.view }
        for r in update.requests { perform(r) }
        #if DEBUG
        if case .scan = view.screen, let message = pendingDelivery {
            pendingDelivery = nil
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { self.apply(self.core.deliver(message: message)) }
        }
        #endif
    }

    /// The app came to the front or left it: the screen's own brightness while away.
    func sceneActive(_ active: Bool) {
        setBright(active && bright)
    }

    /// Full brightness helps a camera read the code on this screen.
    private func setBright(_ on: Bool) {
        if on, brightness == nil {
            brightness = UIScreen.main.brightness
            UIScreen.main.brightness = 1
        } else if !on, let b = brightness {
            UIScreen.main.brightness = b
            brightness = nil
        }
    }

    private func perform(_ r: Request) {
        switch r {
        case let .camera(on):
            cameraOn = on
            if on { camera.start() } else { camera.stop() }
        case let .keepScreenOn(on):
            UIApplication.shared.isIdleTimerDisabled = on
            bright = on
            setBright(on)
        case let .private(on):
            privateScreen = on
        case let .haptic(kind):
            switch kind {
            case .success: UINotificationFeedbackGenerator().notificationOccurred(.success)
            case .warning: UINotificationFeedbackGenerator().notificationOccurred(.warning)
            case .error: UINotificationFeedbackGenerator().notificationOccurred(.error)
            }
        }
    }
}

/// Wi-Fi, Bluetooth and mobile data, as far as iOS lets an app see them.
///
/// - Wi-Fi: iOS has no public API for the Wi-Fi radio itself; a Wi-Fi network path counts
///   as on. A radio that is on but joined to nothing is not seen.
/// - Mobile data: on while the phone is registered on a cellular network (Airplane mode
///   ends that), or while a cellular path exists.
/// - Bluetooth: the power state from CoreBluetooth. Until the person allows the check it is
///   unknown, and unknown counts as on.
final class Radios: NSObject, CBCentralManagerDelegate {
    private let report: (RadioState, RadioState, RadioState, RadioState) -> Void
    private let monitor = NWPathMonitor()
    private let telephony = CTTelephonyNetworkInfo()
    private var bluetoothManager: CBCentralManager?
    private var path: NWPath?
    private var bluetooth = RadioState.unknown
    private var timer: Timer?
    /// The last report: the same one again changes nothing, so it is not sent.
    private var last: [RadioState]?

    init(report: @escaping (RadioState, RadioState, RadioState, RadioState) -> Void) {
        self.report = report
    }

    func start() {
        #if DEBUG && targetEnvironment(simulator)
        // the simulator shares the Mac's network; UI tests declare the phone offline
        if ProcessInfo.processInfo.environment["AIRKEYS_ASSUME_OFFLINE"] == "1" {
            report(.off, .off, .off, .off)
            return
        }
        #endif
        monitor.pathUpdateHandler = { [weak self] p in DispatchQueue.main.async { self?.path = p; self?.send() } }
        monitor.start(queue: DispatchQueue(label: "airkeys.radios"))
        bluetoothManager = CBCentralManager(delegate: self, queue: .main, options: [CBCentralManagerOptionShowPowerAlertKey: false])
        // cellular registration has no change notice worth trusting: look every two seconds
        timer = Timer.scheduledTimer(withTimeInterval: 2, repeats: true) { [weak self] _ in self?.send() }
        send()
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        bluetooth = switch central.state {
        case .poweredOff, .unsupported: .off
        case .poweredOn: .on
        default: .unknown
        }
        send()
    }

    /// What the phone shows of each radio: a Wi-Fi interface (joined, even without internet), a
    /// cellular radio registered on a network, Bluetooth powered — and any network at all
    /// (Wi-Fi, cellular, a cable, a VPN), which must be none.
    private func send() {
        guard let path else {
            last = nil
            return report(.unknown, bluetooth, .unknown, .unknown)
        }
        let interfaces = path.availableInterfaces.map(\.type)
        let wifi: RadioState = interfaces.contains(.wifi) ? .on : .off
        let registered = !(telephony.serviceCurrentRadioAccessTechnology ?? [:]).isEmpty
        let cellular: RadioState = registered || interfaces.contains(.cellular) ? .on : .off
        let network: RadioState = path.status != .unsatisfied || !interfaces.isEmpty ? .on : .off
        guard last != [wifi, bluetooth, cellular, network] else { return }
        last = [wifi, bluetooth, cellular, network]
        report(wifi, bluetooth, cellular, network)
    }
}
