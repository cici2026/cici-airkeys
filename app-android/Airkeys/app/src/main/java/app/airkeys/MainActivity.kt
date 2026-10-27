package app.airkeys

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.util.Base64
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.SystemBarStyle
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import app.airkeys.core.Action
import app.airkeys.core.Screen
import app.airkeys.ui.AirkeysTheme
import app.airkeys.ui.App
import app.airkeys.ui.languageOf
import app.airkeys.ui.tr

class MainActivity : ComponentActivity() {
    private lateinit var shell: Shell

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // the app is always dark: light status and navigation bar icons, whatever the system theme
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
        )
        shell = Shell(this)
        // the system's or the app's own language (Android 13+): a change makes the Activity anew
        shell.dispatch(Action.Language(languageOf(resources.configuration)))
        setContent {
            AirkeysTheme {
                App(shell.view, shell::dispatch, shell::codeFrame) { CameraPreview() }
            }
        }
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                when (shell.view.screen) {
                    is Screen.Welcome, is Screen.Home, is Screen.Offline -> moveTaskToBack(true)
                    else -> shell.dispatch(Action.Back)
                }
            }
        })
        testHooks(intent)
    }

    override fun onStart() {
        super.onStart()
        shell.radios.start()
    }

    override fun onStop() {
        shell.radios.stop()
        super.onStop()
    }

    // only if configChanges ever lists locale: then the Activity is not made anew
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        shell.dispatch(Action.Language(languageOf(newConfig)))
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        testHooks(intent)
    }

    /**
     * Debug builds only: `adb shell am start -n app.airkeys/.MainActivity --es airkeys.deliver <base64 JSON>`
     * hands the open scan screen a message, as if the camera had read it (emulators cannot
     * film a code). It goes through the same review and confirmation.
     */
    private fun testHooks(intent: Intent?) {
        if (!BuildConfig.TEST_HOOKS) return
        val message = intent?.getStringExtra("airkeys.deliver") ?: return
        shell.deliver(Base64.decode(message, Base64.DEFAULT))
    }

    @Composable
    private fun CameraPreview() {
        var granted by remember { mutableStateOf(ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) }
        val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { ok ->
            granted = ok
            if (!ok) {
                shell.dispatch(
                    Action.CameraFailed(
                        tr(
                            shell.view.language,
                            "Airkeys needs the camera to read the web app's codes and SeedQRs. Allow it in the phone's settings, then try again.",
                            "Airkeys 需要摄像头来读取网页助手的动态码和 SeedQR。请在手机设置中允许，然后重试。",
                        ),
                    ),
                )
            }
        }
        LaunchedEffect(shell.cameraOn, granted) { if (shell.cameraOn && !granted) ask.launch(Manifest.permission.CAMERA) }
        if (!shell.cameraOn || !granted) return
        // its errors come on the main thread (the provider's listener)
        val camera = remember {
            ScanCamera(this, shell::frame) { detail ->
                shell.dispatch(Action.CameraFailed(tr(shell.view.language, "The camera could not start: $detail", "摄像头无法启动：$detail")))
            }
        }
        AndroidView(
            factory = { ctx -> PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }.also { camera.start(it) } },
            modifier = Modifier.fillMaxSize(),
        )
        DisposableEffect(Unit) { onDispose { camera.stop() } }
    }
}
