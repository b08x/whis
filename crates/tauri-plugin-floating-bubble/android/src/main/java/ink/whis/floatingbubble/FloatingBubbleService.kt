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
import android.os.IBinder
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.ImageView
import androidx.core.app.NotificationCompat

/**
 * Foreground service that manages the floating bubble overlay.
 *
 * Uses standard Android WindowManager to create a draggable floating bubble.
 * This approach avoids external library dependencies.
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
        
        // Reference to the current service instance for state updates
        @Volatile
        private var instance: FloatingBubbleService? = null
        
        /**
         * Update the bubble's recording state from outside the service.
         */
        fun setRecordingState(recording: Boolean) {
            instance?.updateRecordingState(recording)
        }
    }

    private var windowManager: WindowManager? = null
    private var bubbleView: ImageView? = null
    private var bubbleBackground: GradientDrawable? = null
    private var layoutParams: WindowManager.LayoutParams? = null
    private var isRecording: Boolean = false
    
    // Colors
    private val colorIdle = Color.parseColor("#6366F1")      // Indigo-500
    private val colorRecording = Color.parseColor("#EF4444") // Red-500

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "Service created")
        instance = this
        
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, createNotification())
        
        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        createBubble()
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.d(TAG, "Service destroyed")
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
        val sizePx = (bubbleSize * density).toInt()

        // Create circular background
        bubbleBackground = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(colorIdle)
        }

        // Create bubble view
        bubbleView = ImageView(this).apply {
            background = bubbleBackground

            // Microphone icon
            setImageResource(android.R.drawable.ic_btn_speak_now)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            val padding = (sizePx * 0.2).toInt()
            setPadding(padding, padding, padding, padding)
            setColorFilter(Color.WHITE)

            contentDescription = "Voice input bubble"
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
            x = (bubbleStartX * density).toInt()
            y = (bubbleStartY * density).toInt()
        }

        // Add touch listener for dragging
        bubbleView?.setOnTouchListener(BubbleTouchListener())

        // Add to window
        windowManager?.addView(bubbleView, layoutParams)
        FloatingBubblePlugin.isBubbleVisible = true
        
        Log.d(TAG, "Bubble created at ($bubbleStartX, $bubbleStartY)")
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
                        // It was a click, not a drag
                        handleBubbleClick()
                    } else {
                        // Animate to edge (snap to left or right)
                        animateToEdge()
                    }
                    return true
                }
            }
            return false
        }
    }

    private fun handleBubbleClick() {
        Log.d(TAG, "Bubble clicked")
        // Send event to the Tauri app
        FloatingBubblePlugin.sendBubbleClickEvent()
    }

    private fun animateToEdge() {
        val screenWidth = resources.displayMetrics.widthPixels
        val bubbleWidth = bubbleView?.width ?: 0
        val currentX = layoutParams?.x ?: 0
        
        // Snap to nearest edge
        val targetX = if (currentX + bubbleWidth / 2 < screenWidth / 2) {
            0 // Snap to left
        } else {
            screenWidth - bubbleWidth // Snap to right
        }
        
        layoutParams?.x = targetX
        windowManager?.updateViewLayout(bubbleView, layoutParams)
    }
    
    /**
     * Update the visual state of the bubble based on recording state.
     */
    private fun updateRecordingState(recording: Boolean) {
        if (isRecording == recording) return
        isRecording = recording
        
        Log.d(TAG, "Recording state changed: $recording")
        
        // Update bubble color
        bubbleBackground?.setColor(if (recording) colorRecording else colorIdle)
        
        // Update icon - use stop icon when recording
        bubbleView?.setImageResource(
            if (recording) android.R.drawable.ic_media_pause
            else android.R.drawable.ic_btn_speak_now
        )
        
        // Update notification
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.notify(NOTIFICATION_ID, createNotification())
    }
    
    private fun createNotification(): Notification {
        val title = if (isRecording) "Recording..." else "Whis Voice Input"
        val text = if (isRecording) "Tap bubble to stop" else "Tap the bubble to start recording"
        
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .build()
    }
}
