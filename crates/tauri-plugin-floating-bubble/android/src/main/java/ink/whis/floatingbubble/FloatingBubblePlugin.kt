package ink.whis.floatingbubble

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Options for showing the floating bubble.
 */
@InvokeArg
class BubbleOptions {
    var size: Int = 60
    var startX: Int = 0
    var startY: Int = 100
}

/**
 * Options for setting recording state.
 */
@InvokeArg
class RecordingOptions {
    var recording: Boolean = false
}

/**
 * Tauri plugin for displaying floating bubble overlays on Android.
 *
 * This plugin uses the FloatingBubbleView library to show a draggable bubble
 * that persists across apps using the SYSTEM_ALERT_WINDOW permission.
 */
@TauriPlugin
class FloatingBubblePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "FloatingBubblePlugin"

        // Static flag to track bubble visibility across service restarts
        @Volatile
        var isBubbleVisible: Boolean = false
        
        // Reference to the plugin instance for sending events from the service
        @Volatile
        private var pluginInstance: FloatingBubblePlugin? = null
        
        /**
         * Called from FloatingBubbleService when the bubble is clicked.
         */
        fun sendBubbleClickEvent() {
            pluginInstance?.let { plugin ->
                val event = JSObject()
                event.put("action", "click")
                plugin.trigger("bubble-click", event)
            }
        }
    }
    
    override fun load(webView: android.webkit.WebView) {
        super.load(webView)
        pluginInstance = this
    }

    /**
     * Show the floating bubble with the given options.
     */
    @Command
    fun showBubble(invoke: Invoke) {
        val args = invoke.parseArgs(BubbleOptions::class.java)

        // Check if we have overlay permission
        if (!hasOverlayPermissionInternal()) {
            invoke.reject("Overlay permission not granted. Call requestOverlayPermission first.")
            return
        }

        try {
            // Pass configuration to the service via companion object
            FloatingBubbleService.bubbleSize = args.size
            FloatingBubbleService.bubbleStartX = args.startX
            FloatingBubbleService.bubbleStartY = args.startY

            // Start the floating bubble service
            val intent = Intent(activity, FloatingBubbleService::class.java)
            ContextCompat.startForegroundService(activity, intent)

            isBubbleVisible = true
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Failed to start bubble service: ${e.message}")
        }
    }

    /**
     * Hide the floating bubble.
     */
    @Command
    fun hideBubble(invoke: Invoke) {
        try {
            val intent = Intent(activity, FloatingBubbleService::class.java)
            activity.stopService(intent)
            isBubbleVisible = false
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Failed to stop bubble service: ${e.message}")
        }
    }

    /**
     * Check if the bubble is currently visible.
     */
    @Command
    fun isBubbleVisible(invoke: Invoke) {
        val result = JSObject()
        result.put("visible", isBubbleVisible)
        invoke.resolve(result)
    }

    /**
     * Request the SYSTEM_ALERT_WINDOW permission.
     * Opens system settings if permission is not granted.
     */
    @Command
    fun requestOverlayPermission(invoke: Invoke) {
        if (hasOverlayPermissionInternal()) {
            val result = JSObject()
            result.put("granted", true)
            invoke.resolve(result)
            return
        }

        try {
            val intent = Intent(
                Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                Uri.parse("package:${activity.packageName}")
            )
            activity.startActivity(intent)

            // Note: We can't wait for the result here, so we return false
            // The user should call hasOverlayPermission after returning to the app
            val result = JSObject()
            result.put("granted", false)
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject("Failed to open overlay permission settings: ${e.message}")
        }
    }

    /**
     * Check if the SYSTEM_ALERT_WINDOW permission is granted.
     */
    @Command
    fun hasOverlayPermission(invoke: Invoke) {
        val result = JSObject()
        result.put("granted", hasOverlayPermissionInternal())
        invoke.resolve(result)
    }

    /**
     * Update the bubble's visual state to indicate recording.
     */
    @Command
    fun setBubbleRecording(invoke: Invoke) {
        val args = invoke.parseArgs(RecordingOptions::class.java)
        
        try {
            FloatingBubbleService.setRecordingState(args.recording)
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Failed to update bubble state: ${e.message}")
        }
    }

    /**
     * Internal helper to check overlay permission.
     */
    private fun hasOverlayPermissionInternal(): Boolean {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            Settings.canDrawOverlays(activity)
        } else {
            true // Permission not required on older versions
        }
    }
}
