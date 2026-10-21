package app.airkeys

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.wifi.WifiManager
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.WindowManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
import app.airkeys.core.Action
import app.airkeys.core.AirkeysCore
import app.airkeys.core.CameraFrame
import app.airkeys.core.Haptic
import app.airkeys.core.RadioState
import app.airkeys.core.Request
import app.airkeys.core.Update
import app.airkeys.core.ViewModel

/** The core lives as long as the process: the wallet stays open while the app is in the background. */
object Airkeys {
    val core: AirkeysCore by lazy { AirkeysCore() }
}

/**
 * The Android side of the core: performs what it asks (camera, screen), reports the radios,
 * and holds the newest view for Compose. Which screen, what it says and what may be signed
 * are the core's decisions. Nothing is stored: the phrase lives in the core's memory.
 */
class Shell(private val activity: MainActivity) {
    private val core = Airkeys.core

    var view: ViewModel by mutableStateOf(core.view())
        private set
    /** The core wants the camera on (the scan screen shows the preview then). */
    var cameraOn: Boolean by mutableStateOf(false)
        private set

    /** Sends an action; returns the view it led to (the phrase field reads its text from it). */
    fun dispatch(action: Action): ViewModel {
        apply(core.dispatch(action))
        return view
    }

    fun codeFrame(width: Int, height: Int) = core.codeFrame(width.toUInt(), height.toUInt())

    /** A message that arrived without the camera (test hook, debug builds). */
    fun deliver(message: ByteArray) {
        apply(core.deliver(message))
    }

    private var frames = 0
    private var scanNanos = 0L
    private var statsAt = 0L

    /** A camera frame, on the camera thread. */
    fun frame(frame: CameraFrame) {
        val t0 = System.nanoTime()
        val update = core.scan(frame)
        if (BuildConfig.TEST_HOOKS) stats(frame, System.nanoTime() - t0)
        update ?: return
        activity.runOnUiThread { apply(update) }
    }

    /** Debug builds: once a second, how many frames reached the core and what a scan costs. */
    private fun stats(frame: CameraFrame, nanos: Long) {
        frames++
        scanNanos += nanos
        val now = System.nanoTime()
        if (now - statsAt < 1_000_000_000L) return
        if (statsAt != 0L) Log.d("airkeys-scan", "$frames frames/s at ${frame.width}x${frame.height}, ${scanNanos / frames / 1000} µs per scan (copy + pack + scan)")
        statsAt = now
        frames = 0
        scanNanos = 0
    }

    private fun apply(update: Update) {
        if (update.view.rev >= view.rev) view = update.view
        for (r in update.requests) perform(r)
    }

    private fun perform(r: Request) {
        val window = activity.window
        when (r) {
            is Request.Camera -> cameraOn = r.on
            is Request.KeepScreenOn -> {
                if (r.on) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                // full brightness helps a camera read the code on this screen
                window.attributes = window.attributes.apply {
                    screenBrightness = if (r.on) WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_FULL else WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
                }
            }
            is Request.Private -> {
                // debug builds keep screenshots possible for automated UI checks
                if (BuildConfig.TEST_HOOKS) return
                if (r.on) window.addFlags(WindowManager.LayoutParams.FLAG_SECURE) else window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
            }
            is Request.Haptic -> window.decorView.performHapticFeedback(
                when {
                    Build.VERSION.SDK_INT < 30 -> HapticFeedbackConstants.LONG_PRESS
                    r.kind == Haptic.SUCCESS -> HapticFeedbackConstants.CONFIRM
                    r.kind == Haptic.ERROR -> HapticFeedbackConstants.REJECT
                    else -> HapticFeedbackConstants.LONG_PRESS
                },
            )
        }
    }

    /**
     * Reports Wi-Fi, Bluetooth and mobile data to the core now and on every change. Mobile
     * data counts as off only in airplane mode, which switches the cellular radio off.
     */
    fun watchRadios(context: Context) = Radios(context) { dispatch(it) }.start()
}
