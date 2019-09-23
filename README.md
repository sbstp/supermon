# supermon
**supermon** is a process supervisor meant to run in application containers, such as Docker containers.
It uses a simple YAML specification to spawn and manage the given processes. It
aggregates the output of the processes it spawns (stdout/stderr) and prints it to its own output streams to make their
output available to the container engine.

Its main advantages are:
* **lightweight**: It uses around ~1 MiB of memory at runtime, including the binary shared memory. The binary uses less than 500 KiB on disk.
* **easy to use**: A simple YAML spec is passed to **supermon** to tell it which processes to spawn and manage.
* **easy to install**: You only need to copy a single binary in your container image.

## Usage
Pass your spec as the first argument to the binary:
`/usr/bin/supermon /etc/supermon/spec.yml`

## Spec
* `apps`:
  * `<app-name>`:
    * `exec` **string** path to the executable (required)
    * `args` **list[string]** list of arguments given to the executable (default: `[]`)
    * `env` **list[string]** list of environment variables given to the executable (default: `[]`)
    * `workdir` **string** working directory of the application (default: `"."`)
    * `stdout` **bool** capture stdout, set to `false` to send stdout to `/dev/null` (default: `true`)
    * `stderr` **bool** capture stderr, set to `false` to send stderr to `/dev/null` (default: `true`)
    * `restart` **bool** restart application when it exists (default: `true`)
    * `restartDelay` **int** number of seconds to wait before restarting the application (default: `1`)
    * `disable` **bool** disable the application, do not spawn it (default: `false`)
