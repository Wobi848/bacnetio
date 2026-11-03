package io.github.wobi848.bacnetio

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/**
 * Keeps bacnetio running, also with the screen off (recordings): a foreground service with a
 * notification that starts the program (libbacnetio.so) and starts it again after it ended.
 */
class BacnetioService : Service() {
    private lateinit var wakeLock: PowerManager.WakeLock
    private lateinit var multicastLock: WifiManager.MulticastLock
    @Volatile private var process: Process? = null
    @Volatile private var stopping = false

    override fun onCreate() {
        super.onCreate()
        startForeground(1, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        wakeLock = (getSystemService(POWER_SERVICE) as PowerManager)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "bacnetio:run")
            .apply { acquire() }
        // without it Wi-Fi drops the broadcast answers to Who-Is
        multicastLock = (applicationContext.getSystemService(WIFI_SERVICE) as WifiManager)
            .createMulticastLock("bacnetio")
            .apply {
                setReferenceCounted(false)
                acquire()
            }
        thread(name = "bacnetio") { supervise() }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) stopSelf()
        return START_STICKY
    }

    override fun onDestroy() {
        stopping = true
        process?.destroy()
        multicastLock.release()
        wakeLock.release()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun supervise() {
        while (!stopping) {
            val started = System.currentTimeMillis()
            val running = start()
            process = running
            val code = running.waitFor()
            process = null
            Log.i(TAG, "bacnetio ended with $code")
            // crashed right away again: do not spin
            if (System.currentTimeMillis() - started < 5000) Thread.sleep(5000)
        }
    }

    private fun start(): Process {
        val log = File(filesDir, "bacnetio.log")
        if (log.length() > LOG_MAX) log.renameTo(File(filesDir, "bacnetio.log.1"))
        return ProcessBuilder(
            File(applicationInfo.nativeLibraryDir, "libbacnetio.so").path,
            "--no-browser",
            "--web-port", WEB_PORT.toString(),
            "--data-dir", File(filesDir, "data").path,
        ).apply {
            // an app may not write to Android's /data/local/tmp
            environment()["TMPDIR"] = cacheDir.path
            redirectErrorStream(true)
            redirectOutput(ProcessBuilder.Redirect.appendTo(log))
        }.start()
    }

    private fun notification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, getString(R.string.channel), NotificationManager.IMPORTANCE_LOW)
        )
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE
        )
        val stop = PendingIntent.getService(
            this, 1, Intent(this, BacnetioService::class.java).setAction(ACTION_STOP), PendingIntent.FLAG_IMMUTABLE
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_launcher)
            .setContentTitle(getString(R.string.running))
            .setContentText(getString(R.string.running_text))
            .setContentIntent(open)
            .addAction(Notification.Action.Builder(null, getString(R.string.stop), stop).build())
            .setOngoing(true)
            .build()
    }

    companion object {
        const val WEB_PORT = 8081
        const val ACTION_STOP = "io.github.wobi848.bacnetio.STOP"
        private const val CHANNEL = "run"
        private const val TAG = "bacnetio"
        private const val LOG_MAX = 1024 * 1024L

        /** Does bacnetio answer yet? */
        fun reachable(): Boolean = try {
            val connection = URL("http://127.0.0.1:$WEB_PORT/").openConnection() as HttpURLConnection
            connection.connectTimeout = 500
            connection.readTimeout = 1000
            connection.responseCode > 0
        } catch (e: java.io.IOException) {
            false
        }
    }
}
