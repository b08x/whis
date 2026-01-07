package ink.whis.floatingbubble

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.util.Log
import android.webkit.WebView
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Configuration for a specific bubble state.
 */
@InvokeArg
class StateConfig {
    /**
     * Icon resource name for this state (optional).
     * If not provided, uses the default icon.
     */
    var iconResourceName: String? = null
}

/**
 * Options for showing the floating bubble.
 */
@InvokeArg
class BubbleOptions {
    var size: Int = 60
    var startX: Int = 0
    var startY: Int = 100
    var iconResourceName: String? = null
    var background: String = "#1C1C1C"
    var states: Map<String, StateConfig>? = null
}

/**
 * Options for setting bubble state.
 */
@InvokeArg
class StateOptions {
    var state: String = "idle"
}

/**
 * Tauri plugin for displaying floating bubble overlays on Android.
 *
 * This plugin uses Android's WindowManager to show a draggable bubble
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
        
        // Reference to WebView for emitting events via JavaScript
        @Volatile
        private var webViewInstance: WebView? = null
        
        /**
         * Called from FloatingBubbleService when the bubble is clicked.
         * Emits a global Tauri event via WebView JavaScript evaluation.
         */
        fun sendBubbleClickEvent() {
            val webView = webViewInstance ?: return
            
            // Run on main thread to ensure WebView access is safe
            Handler(Looper.getMainLooper()).post {
                try {
                    // Emit event via Tauri's internal event system (same pattern as plugin-store)
                    val js = """
                        (function() {
                            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                                window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
                                    event: 'floating-bubble://click',
                                    payload: { action: 'click' }
                                }).catch(function(e) { console.error('Failed to emit event:', e); });
                            }
                        })();
                    """.trimIndent()
                    webView.evaluateJavascript(js, null)
                } catch (e: Exception) {
                    Log.e(TAG, "Error emitting bubble-click event", e)
                }
            }
        }
    }
    
    override fun load(webView: WebView) {
        super.load(webView)
        pluginInstance = this
        webViewInstance = webView
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
            FloatingBubbleService.defaultIconResourceName = args.iconResourceName
            FloatingBubbleService.backgroundColor = Color.parseColor(args.background)
            FloatingBubbleService.stateConfigs = args.states ?: emptyMap()

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
     * Parse a color string to an Int, with fallback.
     */
    private fun parseColor(color: String?, default: String): Int {
        return try {
            Color.parseColor(color ?: default)
        } catch (e: Exception) {
            Color.parseColor(default)
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
            FloatingBubbleService.resetState()
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
     * Update the bubble's visual state.
     */
    @Command
    fun setBubbleState(invoke: Invoke) {
        val args = invoke.parseArgs(StateOptions::class.java)

        try {
            Log.d(TAG, "setBubbleState command received, state: ${args.state}")
            FloatingBubbleService.setState(args.state)
            invoke.resolve()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to update bubble state: ${e.message}", e)
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
