import AVFoundation
import AirkeysCore
import SwiftUI

/// The back camera for reading the web app's code: 720p Y′CbCr frames to the core, and a
/// preview to aim with. Late frames are dropped while a scan runs, so scanning paces itself.
final class Camera: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate {
    let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "airkeys.camera")
    private let onFrame: (CameraFrame) -> Void
    private let onError: (String) -> Void

    init(onFrame: @escaping (CameraFrame) -> Void, onError: @escaping (String) -> Void) {
        self.onFrame = onFrame
        self.onError = onError
    }

    func start() {
        AVCaptureDevice.requestAccess(for: .video) { granted in
            guard granted else {
                self.onError("Airkeys needs the camera to read the web app's code. Allow it in Settings › Airkeys, then try again.")
                return
            }
            self.queue.async { self.configure() }
        }
    }

    func stop() {
        queue.async { if self.session.isRunning { self.session.stopRunning() } }
    }

    private func configure() {
        guard let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back),
              let input = try? AVCaptureDeviceInput(device: device) else {
            onError("This device has no camera Airkeys can use.")
            return
        }
        session.beginConfiguration()
        session.sessionPreset = .hd1280x720
        session.inputs.forEach { session.removeInput($0) }
        session.outputs.forEach { session.removeOutput($0) }
        if session.canAddInput(input) { session.addInput(input) }
        let output = AVCaptureVideoDataOutput()
        output.videoSettings = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange]
        output.alwaysDiscardsLateVideoFrames = true
        output.setSampleBufferDelegate(self, queue: queue)
        if session.canAddOutput(output) { session.addOutput(output) }
        session.commitConfiguration()
        // a screen is close and bright: focus near, no stabilisation
        if (try? device.lockForConfiguration()) != nil {
            if device.isAutoFocusRangeRestrictionSupported { device.autoFocusRangeRestriction = .near }
            if device.isFocusModeSupported(.continuousAutoFocus) { device.focusMode = .continuousAutoFocus }
            device.unlockForConfiguration()
        }
        session.startRunning()
    }

    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer, from connection: AVCaptureConnection) {
        guard let px = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        CVPixelBufferLockBaseAddress(px, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(px, .readOnly) }
        guard CVPixelBufferGetPlaneCount(px) == 2,
              let y = CVPixelBufferGetBaseAddressOfPlane(px, 0),
              let c = CVPixelBufferGetBaseAddressOfPlane(px, 1) else { return }
        let (w, h) = (CVPixelBufferGetWidth(px), CVPixelBufferGetHeight(px))
        let yStride = CVPixelBufferGetBytesPerRowOfPlane(px, 0)
        let cStride = CVPixelBufferGetBytesPerRowOfPlane(px, 1)
        let cRows = CVPixelBufferGetHeightOfPlane(px, 1)
        let yData = Data(bytes: y, count: yStride * h)
        let cData = Data(bytes: c, count: cStride * cRows)
        // biplanar 4:2:0: Cb and Cr interleaved in one plane, two bytes per sample
        let frame = CameraFrame(
            width: UInt32(w), height: UInt32(h),
            y: Plane(data: yData, rowStride: UInt32(yStride), pixelStride: 1),
            u: Plane(data: cData, rowStride: UInt32(cStride), pixelStride: 2),
            v: Plane(data: cData.dropFirst(1), rowStride: UInt32(cStride), pixelStride: 2)
        )
        onFrame(frame)
    }
}

struct CameraPreview: UIViewRepresentable {
    let session: AVCaptureSession

    func makeUIView(context: Context) -> PreviewView {
        let v = PreviewView()
        v.preview.session = session
        v.preview.videoGravity = .resizeAspectFill
        return v
    }

    func updateUIView(_ uiView: PreviewView, context: Context) {}

    final class PreviewView: UIView {
        override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }
        var preview: AVCaptureVideoPreviewLayer { layer as! AVCaptureVideoPreviewLayer }
    }
}
