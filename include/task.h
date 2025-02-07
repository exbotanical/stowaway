#ifndef TASK_H
#define TASK_H

#include <libutil/libutil.h>
#include <stdbool.h>
#include <stdlib.h>

#include "utils/xpanic.h"

typedef struct {
  char* root;
  char* target;
  bool  dir;
} task_t;

void free_task(task_t* task);
void run_tasks(array_t* tasks);

#endif /* TASK_H */
