package app.airkeys

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.telephony.TelephonyManager
import android.util.Log
import androidx.core.content.ContextCompat
import app.airkeys.core.Action
import app.airkeys.core.RadioState

/**
 * Whether the phone is offline, as switches and as observed. A radio counts as off only when
 * its switch is off and nothing shows it working: a Bluetooth scanner that exists (the radio is
 * powered — some phones keep Bluetooth "half on" while its switch reads off), a cellular signal,
 * Wi-Fi scanning in the background. And the phone must have no network at all. Every check here
 * works without a permission; a check that cannot be made counts as on.
 *
 * Checked at start, on every radio and network broadcast, and every few seconds.
 */
class Radios(private val context: Context, private val report: (Action.Radios) -> Unit) {
    private val main = Handler(Looper.getMainLooper())
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)
    private val poll = object : Runnable {
        override fun run() {
            check()
            main.postDelayed(this, POLL_MS)
        }
    }

    fun start() {
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_AIRPLANE_MODE_CHANGED)
            addAction(WifiManager.WIFI_STATE_CHANGED_ACTION)
            addAction(BluetoothAdapter.ACTION_STATE_CHANGED)
            addAction(LocationManager.MODE_CHANGED_ACTION)
        }
        // system broadcasts reach a receiver that is not exported
        ContextCompat.registerReceiver(context, object : BroadcastReceiver() {
            override fun onReceive(c: Context, intent: Intent) = check()
        }, filter, ContextCompat.RECEIVER_NOT_EXPORTED)
        // every network, VPNs and restricted ones included
        val any = NetworkRequest.Builder()
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_TRUSTED)
            .build()
        runCatching {
            connectivity?.registerNetworkCallback(any, object : ConnectivityManager.NetworkCallback() {
                override fun onAvailable(network: Network) {
                    main.post { check() }
                }

                override fun onLost(network: Network) {
                    main.post { check() }
                }
            })
        }
        poll.run()
    }

    fun check() {
        val airplane = Settings.Global.getInt(context.contentResolver, Settings.Global.AIRPLANE_MODE_ON, 0) == 1
        report(Action.Radios(wifi(), bluetooth(), cellular(airplane), network()))
    }

    private fun wifi(): RadioState {
        val wifi = context.getSystemService(WifiManager::class.java) ?: return RadioState.UNKNOWN
        val on = runCatching { wifi.isWifiEnabled }.getOrNull() ?: return RadioState.UNKNOWN
        // with "Wi-Fi scanning" on, the radio keeps scanning for networks while Location is on
        val scanning = runCatching {
            wifi.isScanAlwaysAvailable && context.getSystemService(LocationManager::class.java).isLocationEnabled
        }.getOrNull() ?: return RadioState.UNKNOWN
        log("wifi switch=$on scanning=$scanning")
        return when {
            on -> RadioState.ON
            scanning -> RadioState.ACTIVE
            else -> RadioState.OFF
        }
    }

    private fun bluetooth(): RadioState {
        val adapter: BluetoothAdapter? = runCatching { context.getSystemService(BluetoothManager::class.java)?.adapter }.getOrNull()
        // the switch: without a Bluetooth permission (Android 12+) some phones answer isEnabled()
        // with false while it is on; the setting reads 1 when on, 2 when on in Airplane mode
        val switches = listOf(
            runCatching { adapter?.isEnabled }.getOrNull(),
            runCatching { adapter?.state?.let { it != BluetoothAdapter.STATE_OFF } }.getOrNull(),
            runCatching { Settings.Global.getInt(context.contentResolver, Settings.Global.BLUETOOTH_ON) != 0 }.getOrNull(),
        )
        // the radio: a Low Energy scanner exists only while the radio is powered
        val powered = runCatching { adapter?.bluetoothLeScanner != null }.getOrNull()
        log("bluetooth switches=$switches powered=$powered")
        return when {
            switches.any { it == true } -> RadioState.ON
            powered == true -> RadioState.ACTIVE
            adapter == null && switches[2] == false -> RadioState.OFF // no Bluetooth hardware
            switches.all { it == false } && powered == false -> RadioState.OFF
            else -> RadioState.UNKNOWN
        }
    }

    private fun cellular(airplane: Boolean): RadioState {
        if (!airplane) return RadioState.ON
        // Airplane mode switches the cellular radio off; a signal means it is not
        val telephony = context.getSystemService(TelephonyManager::class.java) ?: return RadioState.OFF
        val signal = runCatching { telephony.signalStrength?.cellSignalStrengths?.isNotEmpty() == true }.getOrNull()
        log("cellular airplane=$airplane signal=$signal")
        return when (signal) {
            false -> RadioState.OFF
            true -> RadioState.ACTIVE
            null -> RadioState.UNKNOWN
        }
    }

    @Suppress("DEPRECATION") // allNetworks: every network, not only the default one
    private fun network(): RadioState {
        val networks = runCatching { connectivity?.allNetworks }.getOrNull() ?: return RadioState.UNKNOWN
        log("networks=${networks.size}")
        return if (networks.isEmpty()) RadioState.OFF else RadioState.ON
    }

    private fun log(what: String) {
        if (BuildConfig.TEST_HOOKS) Log.d("airkeys-radio", what)
    }

    private companion object {
        const val POLL_MS = 4_000L
    }
}
