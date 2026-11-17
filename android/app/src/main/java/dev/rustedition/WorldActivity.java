package dev.rustedition;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.net.Uri;
import android.os.Bundle;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class WorldActivity extends Activity {
    static { System.loadLibrary("rust_edition_android"); }

    private static final int OPEN_WORLD = 1;
    private static final int MAX_FILE_BYTES = 64 * 1024 * 1024;
    private static native int[] readPreview(byte[] data);

    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private TextView status;
    private Button open;
    private MapView map;

    @Override protected void onCreate(Bundle savedState) {
        super.onCreate(savedState);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(0xff171e2b);

        LinearLayout toolbar = new LinearLayout(this);
        toolbar.setGravity(Gravity.CENTER_VERTICAL);
        open = new Button(this);
        open.setText("Abrir mundo (.wld)");
        open.setOnClickListener(view -> {
            Intent picker = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            picker.addCategory(Intent.CATEGORY_OPENABLE);
            picker.setType("*/*");
            startActivityForResult(picker, OPEN_WORLD);
        });
        status = new TextView(this);
        status.setTextColor(0xffeeeeee);
        status.setText("Prévia do mapa · Terraria Mobile 1.4.5.8.6");
        toolbar.addView(open);
        toolbar.addView(status);
        root.addView(toolbar);
        map = new MapView();
        root.addView(map, new LinearLayout.LayoutParams(-1, 0, 1));
        setContentView(root);
    }

    @Override protected void onActivityResult(int request, int result, Intent intent) {
        super.onActivityResult(request, result, intent);
        if (request != OPEN_WORLD || result != RESULT_OK || intent == null || intent.getData() == null) {
            return;
        }
        Uri uri = intent.getData();
        open.setEnabled(false);
        status.setText("Lendo o mundo…");
        worker.execute(() -> {
            try {
                int[] pixels = readPreview(readFile(uri));
                if (pixels == null) {
                    throw new IOException("Mundo inválido ou formato diferente de 326.");
                }
                Bitmap bitmap = Bitmap.createBitmap(
                    pixels, 2, pixels[0], pixels[0], pixels[1], Bitmap.Config.ARGB_8888);
                runOnUiThread(() -> {
                    if (isDestroyed()) return;
                    map.show(bitmap);
                    status.setText("Prévia em cores · arraste ou use pinça para ampliar");
                    open.setEnabled(true);
                });
            } catch (IOException | SecurityException error) {
                runOnUiThread(() -> {
                    if (isDestroyed()) return;
                    status.setText(error.getMessage());
                    open.setEnabled(true);
                });
            }
        });
    }

    private byte[] readFile(Uri uri) throws IOException {
        try (InputStream stream = getContentResolver().openInputStream(uri);
             ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            if (stream == null) throw new IOException("Não foi possível abrir o arquivo.");
            byte[] chunk = new byte[64 * 1024];
            int count;
            while ((count = stream.read(chunk)) != -1) {
                if (count > MAX_FILE_BYTES - output.size()) {
                    throw new IOException("O arquivo excede 64 MB.");
                }
                output.write(chunk, 0, count);
            }
            return output.toByteArray();
        }
    }

    @Override protected void onDestroy() {
        worker.shutdownNow();
        super.onDestroy();
    }

    private final class MapView extends View {
        private final Paint paint = new Paint();
        private final RectF target = new RectF();
        private final ScaleGestureDetector pinch;
        private Bitmap bitmap;
        private float scale, minScale, left, top, lastX, lastY;

        MapView() {
            super(WorldActivity.this);
            pinch = new ScaleGestureDetector(WorldActivity.this, new ScaleGestureDetector.SimpleOnScaleGestureListener() {
                @Override public boolean onScale(ScaleGestureDetector detector) {
                    float next = Math.max(minScale, Math.min(minScale * 24, scale * detector.getScaleFactor()));
                    float ratio = next / scale;
                    left = detector.getFocusX() - (detector.getFocusX() - left) * ratio;
                    top = detector.getFocusY() - (detector.getFocusY() - top) * ratio;
                    scale = next;
                    constrain();
                    invalidate();
                    return true;
                }
            });
        }

        void show(Bitmap next) {
            bitmap = next;
            fit();
            invalidate();
        }

        private void fit() {
            if (bitmap == null || getWidth() == 0 || getHeight() == 0) return;
            minScale = Math.min((float)getWidth() / bitmap.getWidth(), (float)getHeight() / bitmap.getHeight());
            scale = minScale;
            constrain();
        }

        private void constrain() {
            left = constrainAxis(left, bitmap.getWidth() * scale, getWidth());
            top = constrainAxis(top, bitmap.getHeight() * scale, getHeight());
        }

        private float constrainAxis(float value, float content, float viewport) {
            if (content <= viewport) return (viewport - content) / 2;
            return Math.max(viewport - content, Math.min(0, value));
        }

        @Override protected void onSizeChanged(int w, int h, int oldW, int oldH) {
            fit();
        }

        @Override protected void onDraw(Canvas canvas) {
            if (bitmap == null) return;
            target.set(left, top, left + bitmap.getWidth() * scale, top + bitmap.getHeight() * scale);
            canvas.drawBitmap(bitmap, null, target, paint);
        }

        @Override public boolean onTouchEvent(MotionEvent event) {
            if (bitmap == null) return false;
            pinch.onTouchEvent(event);
            float x = event.getX(), y = event.getY();
            int action = event.getActionMasked();
            if (action == MotionEvent.ACTION_MOVE && event.getPointerCount() == 1 && !pinch.isInProgress()) {
                left += x - lastX;
                top += y - lastY;
                constrain();
                invalidate();
            }
            int index = action == MotionEvent.ACTION_POINTER_UP && event.getActionIndex() == 0 ? 1 : 0;
            lastX = event.getX(index);
            lastY = event.getY(index);
            return true;
        }
    }
}
