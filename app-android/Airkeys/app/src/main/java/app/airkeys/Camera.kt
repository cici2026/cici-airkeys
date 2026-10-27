package app.airkeys

import android.util.Size
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import app.airkeys.core.CameraFrame
import app.airkeys.core.Plane
import java.util.concurrent.Executors

/**
 * The back camera for reading codes (the web app's, or a SeedQR): a preview to aim with, and
 * 720p Y′CbCr frames handed to the core, which decides everything else. When the core is still
 * busy with one frame, CameraX drops the next (keep-only-latest), so scanning paces itself.
 */
class ScanCamera(private val owner: LifecycleOwner, private val onFrame: (CameraFrame) -> Unit, private val onError: (detail: String) -> Unit) {
    private val worker = Executors.newSingleThreadExecutor()
    private var provider: ProcessCameraProvider? = null
    private var analysis: ImageAnalysis? = null
    private val preview = Preview.Builder().build()
    /** Set by [stop], on the main thread like the start below: a camera stopped before it was ready never starts. */
    private var stopped = false

    fun start(view: PreviewView) {
        val context = view.context
        val future = ProcessCameraProvider.getInstance(context)
        future.addListener({
            if (stopped) return@addListener
            try {
                val p = future.get()
                provider = p
                preview.surfaceProvider = view.surfaceProvider
                val analysis = ImageAnalysis.Builder()
                    .setResolutionSelector(
                        ResolutionSelector.Builder()
                            .setResolutionStrategy(ResolutionStrategy(Size(1280, 720), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER))
                            .build(),
                    )
                    .setOutputImageFormat(ImageAnalysis.OUTPUT_IMAGE_FORMAT_YUV_420_888)
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()
                analysis.setAnalyzer(worker) { image ->
                    val frame = image.use { it.toFrame() }
                    onFrame(frame)
                    // the core has its own copy; this one may hold a SeedQR
                    for (plane in listOf(frame.y, frame.u, frame.v)) plane.data.fill(0)
                }
                this.analysis = analysis
                p.unbindAll()
                p.bindToLifecycle(owner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
            } catch (e: Exception) {
                onError(e.message ?: e.javaClass.simpleName)
            }
        }, ContextCompat.getMainExecutor(context))
    }

    fun stop() {
        stopped = true
        analysis?.clearAnalyzer()
        provider?.unbindAll()
        provider = null
        worker.shutdown()
    }
}

private fun ImageProxy.toFrame(): CameraFrame {
    fun plane(i: Int): Plane {
        val p = planes[i]
        val buffer = p.buffer.duplicate().apply { rewind() }
        val bytes = ByteArray(buffer.remaining()).also { buffer.get(it) }
        return Plane(bytes, p.rowStride.toUInt(), p.pixelStride.toUInt())
    }
    return CameraFrame(width.toUInt(), height.toUInt(), plane(0), plane(1), plane(2))
}
