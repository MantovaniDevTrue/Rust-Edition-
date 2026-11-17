package dev.rustedition;

import android.app.Activity;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.os.Bundle;
import android.util.SparseIntArray;
import android.view.Choreographer;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.widget.TextView;
import android.widget.Toast;
import java.io.File;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public final class MainActivity extends Activity {
    static { System.loadLibrary("rust_edition_android"); }

    private static native long openWorld(String path);
    private static native void closeWorld(long handle);
    private static native boolean saveWorld(long handle, String path);
    private static native void stepWorld(long handle, int axis, boolean jump);
    private static native void editWorld(long handle, int x, int y, boolean place);
    private static native boolean snapshotWorld(long handle, ByteBuffer buffer);

    private String path;
    private GameView game;

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        path = new File(getFilesDir(), "world.re01").getAbsolutePath();
        long handle = openWorld(path);
        if (handle == 0) {
            TextView error = new TextView(this);
            error.setText("Falha ao abrir o mundo salvo. O arquivo foi preservado.");
            setContentView(error);
            return;
        }
        game = new GameView(handle);
        setContentView(game);
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController controller = getWindow().getInsetsController();
            if (controller != null) {
                controller.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                controller.hide(WindowInsets.Type.systemBars());
            }
        } else {
            getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                    | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);
        }
    }

    @Override protected void onResume() {
        super.onResume();
        if (game != null) game.start();
    }

    @Override protected void onPause() {
        if (game != null) {
            game.stop();
            if (!saveWorld(game.handle, path)) {
                Toast.makeText(this, "Falha ao salvar o mundo", Toast.LENGTH_SHORT).show();
            }
        }
        super.onPause();
    }

    @Override protected void onDestroy() {
        if (game != null) {
            game.stop();
            closeWorld(game.handle);
        }
        super.onDestroy();
    }

    private final class GameView extends View implements Choreographer.FrameCallback {
        private final long handle;
        private final Paint paint = new Paint();
        private final ByteBuffer snapshot = ByteBuffer.allocateDirect(16 + 160 * 72).order(ByteOrder.nativeOrder());
        private final int[] colors = {0, 0xff5f913e, 0xff805638, 0xff61697b};
        private final RectF[] buttons = {new RectF(), new RectF(), new RectF(), new RectF()};
        private final SparseIntArray touches = new SparseIntArray();
        private float tileSize, cameraX, cameraY;
        private boolean place, running, jumpPending;
        private long lastFrame, tickDebt;

        GameView(long handle) {
            super(MainActivity.this);
            this.handle = handle;
            setKeepScreenOn(true);
        }

        void start() {
            if (running) return;
            running = true;
            Choreographer.getInstance().postFrameCallback(this);
        }

        void stop() {
            running = false;
            Choreographer.getInstance().removeFrameCallback(this);
            touches.clear();
            jumpPending = false;
            lastFrame = 0;
            tickDebt = 0;
        }

        @Override public void doFrame(long now) {
            if (!running) return;
            if (lastFrame != 0) {
                tickDebt += Math.min(now - lastFrame, 100_000_000L) * 60;
            }
            lastFrame = now;
            int axis = (held(1) ? 1 : 0) - (held(0) ? 1 : 0);
            while (tickDebt >= 1_000_000_000L) {
                stepWorld(handle, axis, jumpPending);
                jumpPending = false;
                tickDebt -= 1_000_000_000L;
            }
            invalidate();
            Choreographer.getInstance().postFrameCallback(this);
        }

        private boolean held(int button) {
            return touches.indexOfValue(button) >= 0;
        }

        @Override protected void onSizeChanged(int width, int height, int oldWidth, int oldHeight) {
            tileSize = Math.max(1f, (float)Math.ceil(height / 22f));
            float size = height * 0.15f;
            float margin = height * 0.08f;
            float bottom = height - margin;
            buttons[0].set(margin, bottom - size, margin + size, bottom);
            buttons[1].set(margin * 1.3f + size, bottom - size, margin * 1.3f + size * 2, bottom);
            buttons[2].set(width - margin - size, bottom - size, width - margin, bottom);
            buttons[3].set(width - margin - size * 1.65f, margin, width - margin, margin + size * 0.75f);
        }

        @Override protected void onDraw(Canvas canvas) {
            if (!snapshotWorld(handle, snapshot)) return;
            int width = snapshot.getInt(0);
            int height = snapshot.getInt(4);
            float x = snapshot.getFloat(8);
            float y = snapshot.getFloat(12);
            cameraX = cameraOffset(x, width, getWidth());
            cameraY = cameraOffset(y, height, getHeight());
            canvas.drawColor(0xff669fc5);

            int right = Math.min(width, (int)Math.ceil((cameraX + getWidth()) / tileSize));
            int bottom = Math.min(height, (int)Math.ceil((cameraY + getHeight()) / tileSize));
            for (int row = (int)(cameraY / tileSize); row < bottom; row++) {
                for (int col = (int)(cameraX / tileSize); col < right; col++) {
                    int tile = snapshot.get(16 + row * width + col);
                    if (tile == 0) continue;
                    paint.setColor(colors[tile]);
                    float left = col * tileSize - cameraX;
                    float top = row * tileSize - cameraY;
                    canvas.drawRect(left, top, left + tileSize, top + tileSize, paint);
                }
            }
            paint.setColor(0xffffd486);
            canvas.drawRect(x * tileSize - cameraX, y * tileSize - cameraY,
                (x + 0.75f) * tileSize - cameraX, (y + 1.65f) * tileSize - cameraY, paint);

            drawButton(canvas, 0, "◀");
            drawButton(canvas, 1, "▶");
            drawButton(canvas, 2, "Pular");
            drawButton(canvas, 3, place ? "Colocar" : "Quebrar");
        }

        private float cameraOffset(float player, int tiles, int viewport) {
            return Math.round(Math.max(0f,
                Math.min(player * tileSize - viewport * 0.5f, tiles * tileSize - viewport)));
        }

        private void drawButton(Canvas canvas, int index, String text) {
            RectF rect = buttons[index];
            paint.setColor(held(index) ? 0xcc182331 : 0xa6182331);
            canvas.drawRoundRect(rect, 12, 12, paint);
            paint.setColor(0xffffffff);
            paint.setTextSize(rect.height() * 0.32f);
            paint.setTextAlign(Paint.Align.CENTER);
            canvas.drawText(text, rect.centerX(), rect.centerY() - (paint.ascent() + paint.descent()) / 2, paint);
        }

        @Override public boolean onTouchEvent(MotionEvent event) {
            int action = event.getActionMasked();
            int index = event.getActionIndex();
            int id = event.getPointerId(index);
            if (action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN) {
                float x = event.getX(index);
                float y = event.getY(index);
                for (int i = 0; i < buttons.length; i++) {
                    if (!buttons[i].contains(x, y)) continue;
                    touches.put(id, i);
                    if (i == 2) jumpPending = true;
                    if (i == 3) place = !place;
                    return true;
                }
                if (tileSize > 0) {
                    editWorld(handle, (int)((cameraX + x) / tileSize),
                        (int)((cameraY + y) / tileSize), place);
                }
            } else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP) {
                touches.delete(id);
            } else if (action == MotionEvent.ACTION_CANCEL) {
                touches.clear();
                jumpPending = false;
            }
            return true;
        }
    }
}
