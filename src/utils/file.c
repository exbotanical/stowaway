#include "utils/file.h"

#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>

#include "utils/xpanic.h"

static inline bool
should_skip_file (const char* fname) {
  return file_is_pointer(fname);
}

array_t*
get_filenames (char* dirpath) {
  DIR* dir;
  if ((dir = opendir(dirpath)) != NULL) {
    array_t* file_names = array_init_or_panic();

    struct dirent* den;
    while ((den = readdir(dir)) != NULL) {
      if (should_skip_file(den->d_name)) {
        continue;
      }

      array_push_or_panic(file_names, s_copy_or_panic(den->d_name));
    }

    closedir(dir);

    return file_names;
  }
  return NULL;
}
