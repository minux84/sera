/* Experimental LG NDL DirectMedia 2 hardware video wallpaper.
 * ABI-compatible with src/hw_wallpaper.rs; video-only, no RGB readback.
 * Luma photo fallback is kept until the TV reports a decoded frame.
 */
#define _GNU_SOURCE
#define NDL_DIRECTMEDIA_API_VERSION 2
#include <NDL_directmedia.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <sys/stat.h>
#include <limits.h>
#include <time.h>

typedef struct {
    int initialized;
    int loaded;
    int64_t frames;
    unsigned long fed;
    int playing;
    int errored;
    unsigned long unknown_events;
    unsigned long raw_negative;
    unsigned long prev_negative;
    int last_unknown_event;
    unsigned long prev_frames;
    unsigned long prev_unknown;
    unsigned long queue_count;
    long long queue_total;
    int queue_min;
    int queue_max;
    // Continuous silent audio keeps the NDL v2 audio master clock running.
    long long audio_pts_ms;
    long long video_max_ms;
    unsigned long audio_fed;
    unsigned long audio_backpressure;
} LumaNDL;
// One 20ms stereo CELT fullband silence frame (TOC config 31, stereo).
static const unsigned char silent_opus_20ms[3]={0xfc,0xff,0xfe};
// Keep audio at most this far ahead of the newest video PTS.
#define AUDIO_AHEAD_MS 40
// NDL calls back on its own threads, also while luma_hw_close() tears the
// session down. The player therefore lives in static storage that is never
// freed (a late callback can never touch freed memory), and `active` is only
// accessed atomically and cleared before NDL is unloaded.
static LumaNDL slot;
static LumaNDL *active=NULL;
static LumaNDL *current(void){return __atomic_load_n(&active,__ATOMIC_ACQUIRE);}
static int claimed=0;
static void release_claim(void){__atomic_store_n(&claimed,0,__ATOMIC_RELEASE);}
static void record(const char *label,int64_t value);

/* NDL DirectMedia is built for game streaming: it always loads Starfish with
 * option.lowDelayMode/isGameMode = true. Low-delay presents each frame as soon
 * as it is decoded, ignoring PTS, so B-frame clips come out in pairs and look
 * like ~15fps. Interpose StarfishMediaAPIs::Load (called by
 * libNDL_directmedia_impl) and turn both flags off for DirectMedia loads only,
 * so the pipeline presents by PTS against the (silent) audio clock. */
#define STARFISH_LOAD "_ZN17StarfishMediaAPIs4LoadEPKcPFvixS1_E"
typedef long (*StarfishLoad)(void *self,const char *json,void (*cb)(int,long long,const char *));
// Copy of json with `"key": true` turned into `"key": false`; NULL if absent.
static char *json_set_false(const char *json,const char *key) {
    char needle[64];
    snprintf(needle,sizeof(needle),"\"%s\"",key);
    const char *at=strstr(json,needle);
    if(!at)return NULL;
    const char *v=at+strlen(needle);
    while(*v==' '||*v==':'||*v=='\t')v++;
    if(strncmp(v,"true",4))return NULL;
    size_t head=(size_t)(v-json),len=strlen(json);
    char *out=malloc(len+2);
    if(!out)return NULL;
    memcpy(out,json,head);
    memcpy(out+head,"false",5);
    memcpy(out+head+5,v+4,len-head-4+1);
    return out;
}
static void log_load(const char *label,const char *json) {
    FILE *f=fopen("/media/developer/com.denisefremov.luma/luma-ndl-load.log",
                  strcmp(label,"ORIGINAL")?"a":"w");
    if(f){fprintf(f,"%s %s\n",label,json);fclose(f);}
}
__attribute__((visibility("default")))
long _ZN17StarfishMediaAPIs4LoadEPKcPFvixS1_E(void *self,const char *json,
                                             void (*cb)(int,long long,const char *)) {
    static StarfishLoad real=NULL;
    if(!real){
        void *h=dlopen("libplayerAPIs.so.1",RTLD_NOW|RTLD_NOLOAD);
        if(!h)h=dlopen("libplayerAPIs.so.1",RTLD_NOW);
        if(h)real=(StarfishLoad)dlsym(h,STARFISH_LOAD);
        if(!real)real=(StarfishLoad)dlsym(RTLD_NEXT,STARFISH_LOAD);
        if(!real){record("LOAD_HOOK_MISSING",-1);return 0;}
    }
    if(!json || !strstr(json,"DIRECTMEDIA") || getenv("LUMA_NDL_LOW_DELAY"))
        return real(self,json,cb);
    log_load("ORIGINAL",json);
    char *a=json_set_false(json,"lowDelayMode");
    char *b=json_set_false(a?a:json,"isGameMode");
    const char *patched=b?b:(a?a:json);
    log_load("PATCHED",patched);
    record("LOW_DELAY_OFF",a!=NULL);
    long rc=real(self,patched,cb);
    free(a);free(b);
    return rc;
}
/* Luma dlopens this library RTLD_LOCAL. Promote it to the global scope so the
 * definition above wins when NDL later dlopens its implementation library. */
__attribute__((constructor)) static void promote_global(void) {
    Dl_info info;
    if(dladdr((void *)promote_global,&info) && info.dli_fname)
        dlopen(info.dli_fname,RTLD_NOW|RTLD_NOLOAD|RTLD_GLOBAL);
}
#define LOG_DIR "/media/developer/com.denisefremov.luma/"
#define EVENTS_LOG LOG_DIR "luma-ndl-events.log"
#define EVENTS_LOG_PREV LOG_DIR "luma-ndl-events.prev.log"
static void record(const char *label,int64_t value) {
    // Same cap as src/logs.rs: past 1 MiB keep one backup, start afresh.
    struct stat st;
    if(stat(EVENTS_LOG,&st)==0 && st.st_size>1024*1024)rename(EVENTS_LOG,EVENTS_LOG_PREV);
    FILE *f=fopen(EVENTS_LOG,"a");
    if(f){fprintf(f,"%s %lld\n",label,(long long)value);fclose(f);}
}
static void resource_released(const char *reason) {
    record("RESOURCE_RELEASED",0);
    LumaNDL *p=current();
    if(p)__atomic_store_n(&p->errored,1,__ATOMIC_RELEASE);
}
static void callback(int type,long long value,const char *str) {
    LumaNDL *p=current();
    if(type==0) {
        if(p){
            int64_t n=__atomic_add_fetch(&p->frames,1,__ATOMIC_SEQ_CST);
            if(n==1 || n%120==0)record("FRAMEREADY",value);
        }
    } else if(type==0x16)record("LOADCOMPLETED",0);
    else if(type==0x1a){record("PLAYING",0);if(p)__atomic_store_n(&p->playing,1,__ATOMIC_RELEASE);}
    else if(type==0x12 || type==0x13){
        record("ERROR",value);
        if(p)__atomic_store_n(&p->errored,1,__ATOMIC_RELEASE);
    } else if(type==0x1c)record("EOS",0);
    else {
        record("UNKNOWN_CALLBACK",type);
        if(p) {
            __atomic_add_fetch(&p->unknown_events,1,__ATOMIC_RELAXED);
            __atomic_store_n(&p->last_unknown_event,type,__ATOMIC_RELAXED);
        }
    }
}
void *luma_hw_open(const char *app_id,int width,int height,const char *codec) {
    if(!app_id||width<=0||height<=0)return NULL;
    // Claim the single NDL session; a second open while one is live fails.
    if(__atomic_exchange_n(&claimed,1,__ATOMIC_ACQ_REL))return NULL;
    LumaNDL *p=&slot;
    memset(p,0,sizeof(*p));
    __atomic_store_n(&active,p,__ATOMIC_RELEASE);
    p->queue_min=INT_MAX;
    record("OPEN",0);
    if(NDL_DirectMediaInit(app_id,resource_released)!=0){
        record("INIT_FAILED",-1);goto fail;
    }
    p->initialized=1;
    NDL_DirectMediaSetAppState(NDL_DIRECTMEDIA_APP_STATE_FOREGROUND);
    NDL_DIRECTMEDIA_DATA_INFO_T info={0};
    info.video.width=width;
    info.video.height=height;
    info.video.type=(codec && !strcmp(codec,"H265"))?NDL_VIDEO_TYPE_H265:NDL_VIDEO_TYPE_H264;
    // NDL v2 requires an audio plane to pace the video correctly. A single
    // silent 5ms stereo Opus packet primes it; no audible sound is emitted.
    // Sample-rate here is in *kHz*, not Hz (webOS NDL ABI).
    info.audio.opus.type=NDL_AUDIO_TYPE_OPUS;
    info.audio.opus.channels=2;
    info.audio.opus.sampleRate=48.0;
    info.audio.opus.streamHeader=NULL;
    if(NDL_DirectMediaLoad(&info,callback)!=0){
        record("LOAD_FAILED",-1);goto fail;
    }
    p->loaded=1;
    const unsigned char silent_opus_5ms[3]={0xec,0xff,0xfe};
    int prime_ok=0;
    for(int i=0;i<8;i++){
        int64_t pts_ms=(int64_t)i*5;
        int result=NDL_DirectAudioPlay((void*)silent_opus_5ms,3,pts_ms);
        if(result==0)prime_ok++;
    }
    record("AUDIO_PRIME_ACCEPTED",prime_ok);
    p->audio_pts_ms=40;
    // NDL v2 self-sizes the hardware video punch-through plane.
    // DirectVideoSetArea is v1-only; on v2 it causes stutter above 1080p.
    record("LOAD_OK",info.video.type);
    return p;
fail:
    __atomic_store_n(&active,NULL,__ATOMIC_RELEASE);
    if(p->loaded)NDL_DirectMediaUnload();
    if(p->initialized)NDL_DirectMediaQuit();
    release_claim();return NULL;
}
int luma_hw_feed(void *opaque,const void *buffer,size_t size,int64_t pts_ns) {
    LumaNDL *p=opaque;
    if(!p || !p->loaded || __atomic_load_n(&p->errored,__ATOMIC_ACQUIRE) || !buffer || !size)return -1;
    int result=NDL_DirectVideoPlay((void*)buffer,(unsigned int)size,pts_ns/1000000);
    if(result==0){
        __atomic_add_fetch(&p->fed,1,__ATOMIC_RELEASE);
        // Without audio after the 40ms prime, the audio master clock stalls
        // and video presentation judders. Feed silence up to the newest video
        // PTS (B-frame PTS is non-monotonic, so track the maximum).
        long long video_ms=pts_ns/1000000;
        if(video_ms>p->video_max_ms)p->video_max_ms=video_ms;
        while(p->audio_pts_ms<=p->video_max_ms+AUDIO_AHEAD_MS){
            if(NDL_DirectAudioPlay((void*)silent_opus_20ms,sizeof(silent_opus_20ms),
                                   p->audio_pts_ms)!=0){
                p->audio_backpressure++;
                break;
            }
            p->audio_fed++;
            p->audio_pts_ms+=20;
        }
        if(p->fed==1 || p->fed%300==0){
            record("AUDIO_FED",(int64_t)p->audio_fed);
            record("AUDIO_BACKPRESSURE",(int64_t)p->audio_backpressure);
        }
        // Sample rendering backlog at a low rate: diagnostic only.
        if(p->fed%5==0){
            int queued=-1;
            if(NDL_DirectVideoGetRenderBufferLength(&queued)==0){
                p->queue_count++;
                p->queue_total+=queued;
                if(queued<p->queue_min)p->queue_min=queued;
                if(queued>p->queue_max)p->queue_max=queued;
            }
        }
        if(p->fed==1 || p->fed%30==0){
            int queued=-1;
            int rc=NDL_DirectVideoGetRenderBufferLength(&queued);
            record("RENDER_QUEUE",rc==0?queued:-1000+rc);
            if(p->fed==1 || p->fed%150==0)record("FEED_OK",p->fed);
        }
        return 0;
    }
    if(result<0)__atomic_add_fetch(&p->raw_negative,1,__ATOMIC_RELAXED);
    if(p->fed==0 || p->fed%120==0)record("FEED_BACKPRESSURE",result);
    return 1;
}
// One compact line per second, capped to 1 MiB with a single backup.
// Called only from the reader thread; callbacks use atomic counters.
void luma_hw_metrics(void *opaque,int index,const char *source,const char *codec,
                     unsigned au,unsigned retries,unsigned errors,double avg_late_ms,
                     double max_late_ms,size_t max_au) {
    LumaNDL *p=opaque;
    if(!p)return;
    const char *path="/media/developer/com.denisefremov.luma/luma-video-metrics.log";
    struct stat st;
    if(stat(path,&st)==0 && st.st_size>1024*1024){
        rename(path,"/media/developer/com.denisefremov.luma/luma-video-metrics.prev.log");
    }
    FILE *f=fopen(path,"a");
    if(!f)return;
    unsigned long frames=__atomic_load_n(&p->frames,__ATOMIC_ACQUIRE);
    unsigned long unknown=__atomic_load_n(&p->unknown_events,__ATOMIC_ACQUIRE);
    unsigned long negative=__atomic_load_n(&p->raw_negative,__ATOMIC_ACQUIRE);
    unsigned long count=p->queue_count;
    struct timespec now={0};
    clock_gettime(CLOCK_MONOTONIC,&now);
    long long elapsed_ms=(long long)now.tv_sec*1000+now.tv_nsec/1000000;
    fprintf(f,"METRIC t_ms=%lld idx=%d src=%s codec=%s au=%u retry=%u error=%u "
            "late_avg_ms=%.2f late_max_ms=%.2f max_au=%lu "
            "q_min=%d q_avg=%.2f q_max=%d q_samples=%lu "
            "frameready=%lu unknown=%lu unknown_type=%d ndl_negative=%lu\n",
            elapsed_ms,index,source,codec,au,retries,errors,avg_late_ms,max_late_ms,
            (unsigned long)max_au,count?p->queue_min:-1,
            count?(double)p->queue_total/count:-1.0,count?p->queue_max:-1,
            count,frames-p->prev_frames,unknown-p->prev_unknown,
            __atomic_load_n(&p->last_unknown_event,__ATOMIC_RELAXED),
            negative-p->prev_negative);
    fclose(f);
    p->prev_frames=frames;
    p->prev_unknown=unknown;
    p->prev_negative=negative;
    p->queue_count=0;
    p->queue_total=0;
    p->queue_min=INT_MAX;
    p->queue_max=0;
}
int luma_hw_set_drop_threshold(void *opaque,int threshold) {
    LumaNDL *p=opaque;
    if(!p || !p->loaded)return -1;
    int rc=NDL_DirectVideoSetFrameDropThreshold(threshold);
    record("DROP_THRESHOLD",rc);
    return rc;
}
int64_t luma_hw_frames_rendered(void *opaque) {
    LumaNDL *p=opaque;
    if(!p)return 0;
    int64_t confirmed=__atomic_load_n(&p->frames,__ATOMIC_SEQ_CST);
    if(confirmed>0)return confirmed;
    // NDL DirectMedia v2 reports PLAYING but may omit FRAMEREADY entirely
    // (verified on webOS 6). After buffered frames have been consumed by the
    // hardware render queue, allow the video plane through the UI alpha.
    int queued=-1;
    if(!__atomic_load_n(&p->errored,__ATOMIC_ACQUIRE)
        && __atomic_load_n(&p->playing,__ATOMIC_ACQUIRE)
        && __atomic_load_n(&p->fed,__ATOMIC_ACQUIRE)>20
        && NDL_DirectVideoGetRenderBufferLength(&queued)==0 && queued<=4)
        return (int64_t)p->fed;
    return 0;
}
int luma_hw_ended(void *opaque) {
    LumaNDL *p=opaque;
    return !p || __atomic_load_n(&p->errored,__ATOMIC_ACQUIRE);
}
void luma_hw_close(void *opaque) {
    LumaNDL *p=opaque;
    if(!p)return;
    record("CLOSE",p->fed);
    // Detach first: callbacks that fire during teardown then see no player
    // and cannot bump the counters of the next session.
    __atomic_store_n(&active,NULL,__ATOMIC_RELEASE);
    if(p->loaded)NDL_DirectMediaUnload();
    if(p->initialized)NDL_DirectMediaQuit();
    release_claim();
}
