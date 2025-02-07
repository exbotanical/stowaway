#ifndef XPANIC_H
#define XPANIC_H

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdnoreturn.h>

#define array_init_or_panic()                                       \
  ({                                                                \
    array_t* __name = array_init();                                 \
    if (!__name) {                                                  \
      xpanic("array_init returned a null pointer - this is a bug"); \
    }                                                               \
    __name;                                                         \
  })

#define s_copy_or_panic(cp)                                   \
  ({                                                          \
    char* __n = s_copy(cp);                                   \
    if (!__n) {                                               \
      xpanic("copy returned a null pointer - this is a bug"); \
    }                                                         \
    __n;                                                      \
  })

#define array_push_or_panic(arr, el)                     \
  if (!array_push(arr, el)) {                            \
    xpanic("array_push failed - this is a bug, or OOM"); \
  }

#define array_get_or_panic(arr, idx)                             \
  ({                                                             \
    void* el = array_get(arr, idx);                              \
    if (!el) {                                                   \
      xpanic("array_get returned null pointer - this is a bug"); \
    }                                                            \
    el;                                                          \
  })

static noreturn void
xpanic (const char* fmt, ...) {
  va_list va;

  va_start(va, fmt);
  vfprintf(stderr, fmt, va);
  va_end(va);

  exit(EXIT_FAILURE);
}

#endif /* XPANIC_H */
