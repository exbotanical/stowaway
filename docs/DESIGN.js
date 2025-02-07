const fs = require('node:fs')
const { chdir } = require('node:process')

/**
 * Where we want to install the packages.
 */
const TARGET = '/tmp/test_target'

/**
 * Where the packages reside.
 */
const ROOT = '/tmp/test_root'
const ROOTS = fs.readdirSync(ROOT).map(p => `${ROOT}/${p}`)

const tasks = []

const cache = new Set()

;(() => {
  if (!fileExists(TARGET)) {
    throw new Error(`Target dir ${TARGET} does not exist`)
  }
  chdir(TARGET)

  processPackages({ rootPaths: ROOTS, targetPath: TARGET })
  runTasks()
})()

function processPackages({ rootPaths, targetPath }) {
  for (const rootPath of rootPaths) {
    processPackage({ rootPath, targetPath })
  }
}

function runTasks() {
  for (const task of tasks) {
    // if (task.dir) {
    //   console.log({ mkdir: task.targetPath })
    //   fs.mkdirSync(task.targetPath)
    // } else {
    //   console.log({ symlink: task.targetPath })
    //   fs.symlinkSync(task.rootPath, task.targetPath)
    // }
    console.log(`>>> ${task.rootPath} -> ${task.targetPath} (${task.dir})`)
  }
}

function processPackage({ rootPath, targetPath, level = 1 }) {
  if (!isDirectory(rootPath)) {
    processNode({ rootPath, targetPath })
    return
  }

  const inPackageRoot = level > 1
  if (inPackageRoot) {
    processNode({ rootPath, targetPath, dir: true })
  }

  const packages = fs.readdirSync(rootPath)

  level++
  for (const p of packages) {
    processPackage({
      rootPath: `${rootPath}/${p}`,
      targetPath: inPackageRoot ? `${targetPath}/${p}` : `${TARGET}/${p}`,
      level,
    })
  }
}

function processNode({ rootPath, targetPath, dir = false }) {
  if (fileExists(targetPath) && fileIsSymlink(targetPath)) {
    if (!fileIsLinkedTo(targetPath, rootPath)) {
      throw new Error(`File ${targetPath} is not owned by stowaway`)
    }
    return
  }

  if (!cache.has(targetPath)) {
    tasks.push({ rootPath, targetPath, dir })
    cache.add(targetPath)
  } else if (!dir) {
    throw new Error(`File ${targetPath} already exists`)
  }
}

// TODO: Option to stowaway terminal node dirs
function isDirectory(filePath) {
  return fs.statSync(filePath).isDirectory()
}

function fileExists(filePath) {
  return fs.existsSync(filePath)
}

function fileIsSymlink(filePath) {
  return fs.statSync(filePath).isSymbolicLink()
}

function fileIsLinkedTo(filePath, link) {
  return fs.readlinkSync(filePath) === link
}
