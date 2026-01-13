package ink.whis.floatingbubble

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.ImageView
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import app.tauri.annotation.InvokeArg

/**
 * Foreground service that manages the floating bubble overlay.
 *
 * Uses standard Android WindowManager to create a draggable floating bubble.
 * Visual states change based on configured icons for each state.
 * Supports drag-to-close with a close zone at the bottom center.
 */
class FloatingBubbleService : Service() {

    companion object {
        private const val TAG = "FloatingBubbleService"
        private const val CHANNEL_ID = "floating_bubble_channel"
        private const val NOTIFICATION_ID = 1001
        private const val CLOSE_ZONE_SIZE = 80
        private const val CLOSE_ZONE_MARGIN = 16

        // Whis palette colors
        private const val COLOR_BG_WEAK = "#1C1C1C"
        private const val COLOR_BG_WEAK_ALPHA = "#CC1C1C1C"
        private const val COLOR_BORDER = "#3D3D3D"
        private const val COLOR_RECORDING = "#FF4444"
        private const val COLOR_RECORDING_ALPHA = "#40FF4444"

        // Configuration passed from the plugin
        var bubbleSize: Int = 60
        var bubbleStartX: Int = 0
        var bubbleStartY: Int = 100
        var defaultIconResourceName: String? = null
        var backgroundColor: Int = Color.parseColor("#1C1C1C")
        var stateConfigs: Map<String, StateConfig> = emptyMap()

        // Reference to the current service instance for state updates
        @Volatile
        private var instance: FloatingBubbleService? = null

        // Store pending state when service isn't ready yet
        @Volatile
        private var pendingState: String? = null

        /**
         * Update the bubble's state from outside the service.
         * Runs on main thread to safely update UI.
         * If service isn't ready, stores the state for later application.
         */
        fun setState(state: String) {
            val service = instance
            if (service == null) {
                pendingState = state
                return
            }
            Handler(Looper.getMainLooper()).post {
                service.updateState(state)
            }
        }

        /**
         * Reset static state when service is fully destroyed.
         */
        fun resetState() {
            pendingState = null
        }
    }

    private var windowManager: WindowManager? = null
    private var bubbleView: ImageView? = null
    private var bubbleBackground: GradientDrawable? = null
    private var layoutParams: WindowManager.LayoutParams? = null
    private var closeZoneParams: WindowManager.LayoutParams? = null
    private var closeZoneView: FrameLayout? = null
    private var closeZoneIcon: ImageView? = null
    private var closeZoneBackground: GradientDrawable? = null
    private var currentStateName: String = "idle"
    private var closeZoneVisible = false
    private var closeZoneActivated = false

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        instance = this

        createNotificationChannel()
        startForeground(NOTIFICATION_ID, createNotification())

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        createCloseZone()
        createBubble()
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
        removeCloseZone()
        removeBubble()
        FloatingBubblePlugin.isBubbleVisible = false
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Floating Bubble",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Voice input bubble service"
                setShowBadge(false)
            }
            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(channel)
        }
    }

    private fun createCloseZone() {
        val density = resources.displayMetrics.density
        val sizePx = (CLOSE_ZONE_SIZE * density).toInt()
        val marginPx = (CLOSE_ZONE_MARGIN * density).toInt()

        closeZoneBackground = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor(COLOR_BG_WEAK_ALPHA))
            setStroke((2 * density).toInt(), Color.parseColor(COLOR_BORDER))
        }

        closeZoneView = FrameLayout(this).apply {
            visibility = View.GONE
            this.background = closeZoneBackground

            // Use custom Whis-branded close icon
            val closeIconResId = resources.getIdentifier(
                "ic_close_zone",
                "drawable",
                packageName
            )

            closeZoneIcon = ImageView(this@FloatingBubbleService).apply {
                val drawable = if (closeIconResId != 0) {
                    ContextCompat.getDrawable(this@FloatingBubbleService, closeIconResId)
                } else {
                    null
                }
                if (drawable != null) {
                    setImageDrawable(drawable)
                } else {
                    setImageResource(android.R.drawable.ic_menu_close_clear_cancel)
                }
                setColorFilter(Color.WHITE)
                val padding = (sizePx * 0.25).toInt()
                setPadding(padding, padding, padding, padding)
            }

            addView(closeZoneIcon, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }

        @Suppress("DEPRECATION")
        val windowType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            WindowManager.LayoutParams.TYPE_PHONE
        }

        closeZoneParams = WindowManager.LayoutParams(
            sizePx,
            sizePx,
            windowType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
            y = marginPx + sizePx
        }

        windowManager?.addView(closeZoneView, closeZoneParams)
    }

    private fun removeCloseZone() {
        closeZoneView?.let {
            try {
                windowManager?.removeView(it)
            } catch (e: Exception) {
                Log.e(TAG, "Error removing close zone view", e)
            }
        }
        closeZoneView = null
    }

    private fun showCloseZone() {
        if (closeZoneVisible) return
        closeZoneVisible = true
        closeZoneView?.visibility = View.VISIBLE
    }

    private fun hideCloseZone() {
        if (!closeZoneVisible) return
        closeZoneVisible = false
        closeZoneActivated = false
        closeZoneView?.visibility = View.GONE
        closeZoneBackground?.setColor(Color.parseColor(COLOR_BG_WEAK_ALPHA))
        closeZoneBackground?.setStroke((2 * resources.displayMetrics.density).toInt(), Color.parseColor(COLOR_BORDER))
    }

    private fun updateCloseZoneFeedback(isClose: Boolean) {
        if (isClose == closeZoneActivated) return
        closeZoneActivated = isClose
        val density = resources.displayMetrics.density
        if (isClose) {
            closeZoneBackground?.setColor(Color.parseColor(COLOR_RECORDING_ALPHA))
            closeZoneBackground?.setStroke((3 * density).toInt(), Color.parseColor(COLOR_RECORDING))
            closeZoneIcon?.setColorFilter(Color.parseColor(COLOR_RECORDING))
        } else {
            closeZoneBackground?.setColor(Color.parseColor(COLOR_BG_WEAK_ALPHA))
            closeZoneBackground?.setStroke((2 * density).toInt(), Color.parseColor(COLOR_BORDER))
            closeZoneIcon?.setColorFilter(Color.WHITE)
        }
    }

    private fun createBubble() {
        val density = resources.displayMetrics.density
        val sizePx = (Companion.bubbleSize * density).toInt()
        val currentBackgroundColor = Companion.backgroundColor
        val currentIconResourceName = Companion.defaultIconResourceName

        // Create circular background with configured color
        bubbleBackground = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(currentBackgroundColor)
        }

        // Create bubble view with default icon
        bubbleView = ImageView(this).apply {
            background = bubbleBackground

            // Load icon by resource name, fallback to default
            val iconResId = if (!currentIconResourceName.isNullOrEmpty()) {
                resources.getIdentifier(
                    currentIconResourceName,
                    "drawable",
                    packageName
                )
            } else {
                0
            }

            if (iconResId != 0) {
                try {
                    val iconDrawable = ContextCompat.getDrawable(
                        this@FloatingBubbleService,
                        iconResId
                    )
                    setImageDrawable(iconDrawable)
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to load icon: $currentIconResourceName", e)
                    loadDefaultIcon()
                }
            } else {
                // Try plugin's default icon, then fallback to system icon
                loadDefaultIcon()
            }

            scaleType = ImageView.ScaleType.CENTER_INSIDE
            val padding = (sizePx * 0.22).toInt()
            setPadding(padding, padding, padding, padding)

            contentDescription = "Floating bubble"
        }

        // Window layout params for overlay
        @Suppress("DEPRECATION")
        val windowType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            WindowManager.LayoutParams.TYPE_PHONE
        }

        layoutParams = WindowManager.LayoutParams(
            sizePx,
            sizePx,
            windowType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = (Companion.bubbleStartX * density).toInt()
            y = (Companion.bubbleStartY * density).toInt()
        }

        // Add touch listener for dragging
        bubbleView?.setOnTouchListener(BubbleTouchListener())

        // Add to window
        windowManager?.addView(bubbleView, layoutParams)
        FloatingBubblePlugin.isBubbleVisible = true

        // Apply any pending state that was set before service was ready
        val pending = pendingState
        if (pending != null) {
            pendingState = null
            updateState(pending)
        } else {
            currentStateName = "idle"
        }
    }

    /**
     * Load the plugin's default icon or fallback to system icon.
     */
    private fun ImageView.loadDefaultIcon() {
        // Try plugin's default icon first
        val defaultResId = resources.getIdentifier(
            "ic_floating_bubble_default",
            "drawable",
            packageName
        )

        if (defaultResId != 0) {
            try {
                val defaultDrawable = ContextCompat.getDrawable(
                    this@FloatingBubbleService,
                    defaultResId
                )
                setImageDrawable(defaultDrawable)
                return
            } catch (e: Exception) {
                // Fall through to system icon
            }
        }

        // Fallback to system icon
        setImageResource(android.R.drawable.ic_btn_speak_now)
    }

    private fun removeBubble() {
        bubbleView?.let {
            try {
                windowManager?.removeView(it)
            } catch (e: Exception) {
                Log.e(TAG, "Error removing bubble view", e)
            }
        }
        bubbleView = null
    }

    /**
     * Touch listener that handles dragging the bubble.
     */
    private inner class BubbleTouchListener : View.OnTouchListener {

        private var initialX = 0
        private var initialY = 0
        private var initialTouchX = 0f
        private var initialTouchY = 0f
        private var isDragging = false
        private val clickThreshold = 10 // pixels

        override fun onTouch(view: View, event: MotionEvent): Boolean {
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    initialX = layoutParams?.x ?: 0
                    initialY = layoutParams?.y ?: 0
                    initialTouchX = event.rawX
                    initialTouchY = event.rawY
                    isDragging = false
                    showCloseZone()
                    return true
                }
                MotionEvent.ACTION_MOVE -> {
                    val deltaX = (event.rawX - initialTouchX).toInt()
                    val deltaY = (event.rawY - initialTouchY).toInt()

                    if (kotlin.math.abs(deltaX) > clickThreshold ||
                        kotlin.math.abs(deltaY) > clickThreshold) {
                        isDragging = true
                    }

                    layoutParams?.x = initialX + deltaX
                    layoutParams?.y = initialY + deltaY
                    windowManager?.updateViewLayout(bubbleView, layoutParams)

                    updateCloseZoneFeedback(isNearCloseZone())
                    return true
                }
                MotionEvent.ACTION_UP -> {
                    hideCloseZone()
                    if (!isDragging) {
                        handleBubbleClick()
                    } else {
                        if (isInCloseZone()) {
                            handleCloseBubble()
                        } else {
                            animateToEdge()
                        }
                    }
                    return true
                }
            }
            return false
        }

        /**
         * Calculate distance between bubble center and close zone center.
         * Returns null if either view is unavailable.
         */
        private fun getDistanceToCloseZone(): Double? {
            val bubble = bubbleView ?: return null
            val closeZone = closeZoneView ?: return null

            val bubbleLocation = IntArray(2)
            val closeZoneLocation = IntArray(2)
            bubble.getLocationOnScreen(bubbleLocation)
            closeZone.getLocationOnScreen(closeZoneLocation)

            val bubbleCenterX = bubbleLocation[0] + bubble.width / 2
            val bubbleCenterY = bubbleLocation[1] + bubble.height / 2
            val closeZoneCenterX = closeZoneLocation[0] + closeZone.width / 2
            val closeZoneCenterY = closeZoneLocation[1] + closeZone.height / 2

            return kotlin.math.sqrt(
                Math.pow((bubbleCenterX - closeZoneCenterX).toDouble(), 2.0) +
                Math.pow((bubbleCenterY - closeZoneCenterY).toDouble(), 2.0)
            )
        }

        private fun getCombinedRadius(): Double {
            val bubble = bubbleView ?: return 0.0
            val closeZone = closeZoneView ?: return 0.0
            return (closeZone.width / 2 + bubble.width / 2).toDouble()
        }

        private fun isInCloseZone(): Boolean {
            val distance = getDistanceToCloseZone() ?: return false
            return distance < getCombinedRadius() * 0.7
        }

        private fun isNearCloseZone(): Boolean {
            val distance = getDistanceToCloseZone() ?: return false
            return distance < getCombinedRadius() * 1.2
        }
    }

    private fun handleBubbleClick() {
        FloatingBubblePlugin.sendBubbleClickEvent()
    }

    private fun handleCloseBubble() {
        FloatingBubblePlugin.sendCloseEvent()
        hideBubble()
    }

    private fun hideBubble() {
        try {
            val intent = Intent(this, FloatingBubbleService::class.java)
            stopService(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Error hiding bubble", e)
        }
    }

    private fun animateToEdge() {
        val screenWidth = resources.displayMetrics.widthPixels
        val bubbleWidth = bubbleView?.width ?: 0
        val currentX = layoutParams?.x ?: 0

        val targetX = if (currentX + bubbleWidth / 2 < screenWidth / 2) {
            0
        } else {
            screenWidth - bubbleWidth
        }

        layoutParams?.x = targetX
        windowManager?.updateViewLayout(bubbleView, layoutParams)
    }

    /**
     * Update the visual state of the bubble.
     * Changes the icon based on state configuration.
     */
    private fun updateState(stateName: String) {
        if (currentStateName == stateName) return
        currentStateName = stateName

        // Determine icon: state-specific icon -> default icon -> system fallback
        val config = Companion.stateConfigs[stateName]
        val iconName = config?.iconResourceName ?: Companion.defaultIconResourceName

        if (iconName != null) {
            val iconResId = resources.getIdentifier(iconName, "drawable", packageName)
            if (iconResId != 0) {
                val iconDrawable = ContextCompat.getDrawable(this, iconResId)
                bubbleView?.setImageDrawable(iconDrawable)
            } else {
                Log.w(TAG, "State icon resource not found: $iconName")
            }
        }

        // Update notification
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.notify(NOTIFICATION_ID, createNotification())
    }

    private fun createNotification(): Notification {
        val (title, text) = when (currentStateName.lowercase()) {
            "recording" -> "Recording..." to "Tap bubble to stop"
            "processing" -> "Processing..." to "Transcribing your voice"
            else -> "Floating Bubble" to "Tap the bubble to interact"
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .build()
    }
}
