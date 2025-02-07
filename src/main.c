#include <dirent.h>
#include <errno.h>
#include <libhash/libhash.h>
#include <libutil/libutil.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "task.h"
#include "utils/file.h"
#include "utils/xmalloc.h"
#include "utils/xpanic.h"

static const char* target_dir = "/tmp/test_target";
static const char* root_dir   = "/tmp/test_root";

// stowaway --target=$HOME
// stowaway --target=$HOME --delete

static hash_set* cache;
static array_t*  packages;
static array_t*  tasks;

static void
cleanup (void) {
  array_free(packages, free);
  array_free(tasks, free);
  hs_delete_set(cache);
}

static void
process_node (const char* package_node, const char* target_path, bool is_dir) {
  if (file_exists(target_path) && file_is_symlink(target_path)) {
    if (!file_is_symlinked_to(target_path, package_node)) {
      // TODO: Pass progname dyn
      xpanic("File %s is not owned by stowaway\n", target_path);
    }
    return;
  }

  if (!hs_contains(cache, target_path)) {
    task_t* task = (task_t*)xmalloc(sizeof(task_t));
    task->dir    = is_dir;
    task->root   = s_copy_or_panic(package_node);
    task->target = s_copy_or_panic(target_path);

    array_push_or_panic(tasks, task);
    hs_insert(cache, target_path);

  } else if (!is_dir) {
    xpanic("File %s already exists\n", target_path);
  }
}

static void
process_package (const char* package, const char* target_path, unsigned int level) {
  if (!file_is_directory(package)) {
    process_node(package, target_path, false);
    return;
  }

  int in_package_root = level > 1;
  if (in_package_root) {
    process_node(package, target_path, true);
  }

  array_t* subpackages = get_filenames(package);

  level++;
  foreach (subpackages, i) {
    const char* subpackage      = array_get_or_panic(subpackages, i);
    char*       subpackage_path = s_fmt("%s/%s", package, subpackage);
    char* normalized_target_path = s_fmt("%s/%s", in_package_root ? target_path : target_dir, subpackage);

    process_package(subpackage_path, normalized_target_path, level);

    free(subpackage_path);
    free(normalized_target_path);
  }
}

int
main (int argc, char* argv[]) {
  if (!file_exists(target_dir)) {
    xpanic("Target directory %s does not exist\n", target_dir);
  }

  if (chdir(target_dir) != 0) {
    xpanic("failed to chdir (reason: %s)\n", strerror(errno));
  }

  if (atexit(cleanup) != 0) {
    xpanic("failed to call atexit (reason: %s)\n", strerror(errno));
  }

  packages = get_filenames(root_dir);
  tasks    = array_init_or_panic();
  cache    = hs_init(HS_DEFAULT_CAPACITY);

  foreach (packages, i) {
    const char* package            = array_get_or_panic(packages, i);
    char*       normalized_package = s_fmt("%s/%s", root_dir, package);
    process_package(normalized_package, target_dir, 1);
    free(normalized_package);
  }

  run_tasks(tasks);

  return EXIT_SUCCESS;
}
