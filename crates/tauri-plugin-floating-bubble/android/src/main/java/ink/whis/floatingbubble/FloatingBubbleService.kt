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
import android.widget.ImageView
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

/**
 * Configuration for a specific bubble state.
 */
data class StateConfig(
    val iconResourceName: String? = null
)

/**
 * Foreground service that manages the floating bubble overlay.
 *
 * Uses standard Android WindowManager to create a draggable floating bubble.
 * Visual states (background stays constant, only icon color changes):
 * - Idle: Default icon color (configurable, default white)
 * - Recording: Recording icon color (configurable, default red)
 * - Processing: Processing icon color (configurable, default gold)
 */
class FloatingBubbleService : Service() {

    companion object {
        private const val TAG = "FloatingBubbleService"
        private const val CHANNEL_ID = "floating_bubble_channel"
        private const val NOTIFICATION_ID = 1001

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
            Log.d(TAG, "setState called with: $state, instance: $service")
            if (service == null) {
                // Store for later - will be applied when service starts
                Log.d(TAG, "Instance is null, storing pending state: $state")
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
    private var currentStateName: String = "idle"

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        instance = this
        
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, createNotification())
        
        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        createBubble()
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
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
            Log.d(TAG, "Applying pending state on bubble creation: $pending")
            pendingState = null
            updateState(pending)
        } else {
            // Apply initial idle state
            Log.d(TAG, "No pending state, applying initial IDLE")
            applyIdleState()
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
                    return true
                }
                MotionEvent.ACTION_UP -> {
                    if (!isDragging) {
                        handleBubbleClick()
                    } else {
                        animateToEdge()
                    }
                    return true
                }
            }
            return false
        }
    }

    private fun handleBubbleClick() {
        FloatingBubblePlugin.sendBubbleClickEvent()
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
        Log.d(TAG, "updateState called: $stateName, current: $currentStateName")
        if (currentStateName == stateName) {
            Log.d(TAG, "State unchanged, skipping")
            return
        }
        currentStateName = stateName

        // Get state configuration
        val config = Companion.stateConfigs[stateName]
        Log.d(TAG, "State config for '$stateName': $config")

        // Determine icon: state-specific icon -> default icon -> system fallback
        val iconName = config?.iconResourceName ?: Companion.defaultIconResourceName

        if (iconName != null) {
            // Load and set state-specific icon
            val iconResId = resources.getIdentifier(iconName, "drawable", packageName)
            if (iconResId != 0) {
                try {
                    val iconDrawable = ContextCompat.getDrawable(this, iconResId)
                    bubbleView?.setImageDrawable(iconDrawable)
                    Log.d(TAG, "Loaded state icon: $iconName (resId: $iconResId)")
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to load state icon: $iconName", e)
                }
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
