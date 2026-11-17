#include <jni.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

void *re_open(const char *path);
void re_close(void *handle);
bool re_save(void *handle, const char *path);
void re_step(void *handle, int32_t axis, bool jump);
bool re_edit(void *handle, int32_t x, int32_t y, bool place);
bool re_snapshot(const void *handle, uint8_t *output, size_t length);

JNIEXPORT jlong JNICALL Java_dev_rustedition_MainActivity_openWorld(
    JNIEnv *env, jclass type, jstring path) {
    const char *name = (*env)->GetStringUTFChars(env, path, NULL);
    if (!name) return 0;
    void *handle = re_open(name);
    (*env)->ReleaseStringUTFChars(env, path, name);
    return (jlong)(intptr_t)handle;
}

JNIEXPORT void JNICALL Java_dev_rustedition_MainActivity_closeWorld(
    JNIEnv *env, jclass type, jlong handle) {
    re_close((void *)(intptr_t)handle);
}

JNIEXPORT jboolean JNICALL Java_dev_rustedition_MainActivity_saveWorld(
    JNIEnv *env, jclass type, jlong handle, jstring path) {
    const char *name = (*env)->GetStringUTFChars(env, path, NULL);
    if (!name) return JNI_FALSE;
    bool saved = re_save((void *)(intptr_t)handle, name);
    (*env)->ReleaseStringUTFChars(env, path, name);
    return saved;
}

JNIEXPORT void JNICALL Java_dev_rustedition_MainActivity_stepWorld(
    JNIEnv *env, jclass type, jlong handle, jint axis, jboolean jump) {
    re_step((void *)(intptr_t)handle, axis, jump != JNI_FALSE);
}

JNIEXPORT void JNICALL Java_dev_rustedition_MainActivity_editWorld(
    JNIEnv *env, jclass type, jlong handle, jint x, jint y, jboolean place) {
    re_edit((void *)(intptr_t)handle, x, y, place != JNI_FALSE);
}

JNIEXPORT jboolean JNICALL Java_dev_rustedition_MainActivity_snapshotWorld(
    JNIEnv *env, jclass type, jlong handle, jobject buffer) {
    jlong capacity = (*env)->GetDirectBufferCapacity(env, buffer);
    return capacity >= 0 && re_snapshot((const void *)(intptr_t)handle,
        (*env)->GetDirectBufferAddress(env, buffer), (size_t)capacity);
}

int32_t *re_world_preview(const uint8_t *input, size_t input_len, size_t *output_len);
void re_drop_preview(int32_t *pixels, size_t length);

JNIEXPORT jintArray JNICALL Java_dev_rustedition_WorldActivity_readPreview(
    JNIEnv *env, jclass type, jbyteArray input) {
    jsize input_len = (*env)->GetArrayLength(env, input);
    if (input_len == 0) return NULL;
    jbyte *bytes = (*env)->GetByteArrayElements(env, input, NULL);
    if (!bytes) return NULL;
    size_t output_len = 0;
    int32_t *pixels = re_world_preview((const uint8_t *)bytes, (size_t)input_len, &output_len);
    (*env)->ReleaseByteArrayElements(env, input, bytes, JNI_ABORT);
    if (!pixels) return NULL;
    jintArray result = (*env)->NewIntArray(env, (jsize)output_len);
    if (result) {
        (*env)->SetIntArrayRegion(env, result, 0, (jsize)output_len, (const jint *)pixels);
    }
    re_drop_preview(pixels, output_len);
    return result;
}
