package io.github.wobi848.bacnetio

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.content.ContentValues
import android.content.Intent
import android.os.Bundle
import android.provider.MediaStore
import android.webkit.URLUtil
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/** The bacnetio web UI in the whole window; bacnetio itself runs in BacnetioService. */
class MainActivity : Activity() {
    private lateinit var web: WebView
    private val home = "http://127.0.0.1:${BacnetioService.WEB_PORT}/"

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 0)
        startForegroundService(Intent(this, BacnetioService::class.java))

        web = WebView(this)
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.webViewClient = object : WebViewClient() {
            // other links in the browser, bacnetio itself here
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (request.url.toString().startsWith(home)) return false
                startActivity(Intent(Intent.ACTION_VIEW, request.url))
                return true
            }
        }
        web.setDownloadListener { url, _, disposition, mime, _ ->
            download(url, URLUtil.guessFileName(url, disposition, mime), mime)
        }
        web.loadData(getString(R.string.starting), "text/plain; charset=utf-8", null)
        setContentView(web)
        waitForBacnetio()
    }

    private fun waitForBacnetio() = thread {
        repeat(60) {
            if (BacnetioService.reachable()) {
                runOnUiThread { web.loadUrl(home) }
                return@thread
            }
            Thread.sleep(500)
        }
        runOnUiThread { web.loadData(getString(R.string.failed), "text/plain; charset=utf-8", null) }
    }

    /** CSV exports go to Downloads. */
    private fun download(url: String, name: String, mime: String?) = thread {
        val message = try {
            val connection = URL(url).openConnection() as HttpURLConnection
            val values = ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, name)
                put(MediaStore.Downloads.MIME_TYPE, mime ?: "application/octet-stream")
            }
            val target = contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
                ?: throw IOException(name)
            connection.inputStream.use { input ->
                contentResolver.openOutputStream(target)!!.use { input.copyTo(it) }
            }
            getString(R.string.saved, name)
        } catch (e: IOException) {
            getString(R.string.save_failed, e.message)
        }
        runOnUiThread { Toast.makeText(this, message, Toast.LENGTH_LONG).show() }
    }

    @Deprecated("Activity without AndroidX")
    override fun onBackPressed() {
        if (web.canGoBack()) web.goBack() else super.onBackPressed()
    }
}
