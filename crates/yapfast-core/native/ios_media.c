/* The iPhone build's video files and watched streams: libwebrtc for iOS
 * carries no FFmpeg, so these stand in for video.c and decode.c. Opening a
 * video or a stream's decoder fails, and Yapfast says it can't play it. See
 * docs/ios.md. */
#include <stddef.h>
#include <stdint.h>

struct yapfast_video;
struct yapfast_video_info;
struct yapfast_decoder;
struct yapfast_picture;

struct yapfast_video *yapfast_video_open(void *source, void *read, void *seek_to,
                                         unsigned threads, int sound,
                                         struct yapfast_video_info *info) {
  (void)source; (void)read; (void)seek_to; (void)threads; (void)sound; (void)info;
  return NULL;
}

int yapfast_video_next(struct yapfast_video *video, int64_t *time_ms, int pictures) {
  (void)video; (void)time_ms; (void)pictures;
  return -1;
}

int yapfast_video_picture(struct yapfast_video *video, unsigned char *rgba, int width,
                          int height) {
  (void)video; (void)rgba; (void)width; (void)height;
  return -1;
}

const float *yapfast_video_sound(const struct yapfast_video *video, int *frames, int *rate,
                                 int *channels) {
  (void)video;
  *frames = 0;
  *rate = 0;
  *channels = 0;
  return NULL;
}

int yapfast_video_seek(struct yapfast_video *video, int64_t time_ms) {
  (void)video; (void)time_ms;
  return -1;
}

void yapfast_video_free(struct yapfast_video *video) { (void)video; }

struct yapfast_decoder *yapfast_decoder_new(int codec, unsigned threads) {
  (void)codec; (void)threads;
  return NULL;
}

int yapfast_decoder_decode(struct yapfast_decoder *decoder, const unsigned char *data,
                           size_t size, struct yapfast_picture *picture) {
  (void)decoder; (void)data; (void)size; (void)picture;
  return -1;
}

void yapfast_decoder_free(struct yapfast_decoder *decoder) { (void)decoder; }
