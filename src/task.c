#include "task.h"

void
free_task (task_t* task) {
  free(task->root);
  free(task->target);
  free(task);
}

void
run_tasks (array_t* tasks) {
  foreach (tasks, i) {
    task_t* task = array_get_or_panic(tasks, i);
    printf(">>> %s -> %s (%s)\n", task->root, task->target, task->dir ? "dir" : "file");
  }
}
