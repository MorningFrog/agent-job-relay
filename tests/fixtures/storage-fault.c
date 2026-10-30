// Linux test-only I/O faults, restricted to an explicit fixture profile and
// activation file. The production runtime has no fault-injection switches.
#define _GNU_SOURCE
#define _LARGEFILE64_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

static int fail(int fd, int sync) {
    const char *root = getenv("AJR_TEST_FAULT_ROOT");
    const char *flag = getenv("AJR_TEST_FAULT_FLAG");
    const char *mode = getenv("AJR_TEST_FAULT_MODE");
    if (!root || !flag || !mode || access(flag, F_OK)) return 0;
    char link[64], path[4096];
    snprintf(link, sizeof(link), "/proc/self/fd/%d", fd);
    ssize_t n = readlink(link, path, sizeof(path)-1);
    if (n < 0) return 0;
    path[n] = 0;
    size_t prefix = strlen(root), len = (size_t)n;
    if (strncmp(path, root, prefix) || path[prefix] != '/') return 0;
    int log = len >= 11 && !strcmp(path+len-11, "/stdout.log");
    int wal = len >= 18 && !strcmp(path+len-18, "/relay.sqlite3-wal");
    if (!((log && !strcmp(mode, sync ? "log_sync" : "log_write")) ||
          (wal && !sync && !strcmp(mode, "wal_write")))) return 0;
    const char *evidence = getenv("AJR_TEST_FAULT_EVIDENCE");
    if (evidence) {
        int out = (int)syscall(SYS_openat, AT_FDCWD, evidence, O_WRONLY|O_CREAT|O_APPEND|O_CLOEXEC, 0600);
        if (out >= 0) { syscall(SYS_write, out, "ENOSPC\n", 7); syscall(SYS_close, out); }
    }
    errno = ENOSPC;
    return 1;
}

ssize_t write(int fd, const void *buf, size_t n) {
    if (fail(fd, 0)) return -1;
    ssize_t (*real)(int,const void*,size_t) = dlsym(RTLD_NEXT,"write");
    return real(fd,buf,n);
}
ssize_t pwrite(int fd, const void *buf, size_t n, off_t at) {
    if (fail(fd, 0)) return -1;
    ssize_t (*real)(int,const void*,size_t,off_t) = dlsym(RTLD_NEXT,"pwrite");
    return real(fd,buf,n,at);
}
ssize_t pwrite64(int fd, const void *buf, size_t n, off64_t at) {
    if (fail(fd, 0)) return -1;
    ssize_t (*real)(int,const void*,size_t,off64_t) = dlsym(RTLD_NEXT,"pwrite64");
    return real(fd,buf,n,at);
}
int fsync(int fd) {
    if (fail(fd, 1)) return -1;
    int (*real)(int) = dlsym(RTLD_NEXT,"fsync");
    return real(fd);
}
int fdatasync(int fd) {
    if (fail(fd, 1)) return -1;
    int (*real)(int) = dlsym(RTLD_NEXT,"fdatasync");
    return real(fd);
}
