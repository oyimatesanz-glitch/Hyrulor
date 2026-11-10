## Help

- Print the basic help screen

`python main.py --help`

```bash
usage: main.py [-h] [--target-image IMAGE_REF] [--peek-layer PEEK_LAYER] [--save-all]
               [--log-file LOG_FILE] [--simple-output] [--bulk-peek] [--carve-file CARVE_FILE]
               [--carve-layer CARVE_LAYER] [--output-dir OUTPUT_DIR] [--quiet] [--interactive]
               [--arch ARCH] [--force] [--hide-build] [--api]

Explore and download individual Docker image layers.

options:
  -h, --help            show this help message and exit
  --target-image IMAGE_REF, -t IMAGE_REF
                        Image (user/repo:tag) to inspect
  --peek-layer PEEK_LAYER
                        Peek layer(s): 'all' for all layers, or integer index for specific layer
  --save-all            Download all layers and exit (no peek listings)
  --log-file LOG_FILE, -l LOG_FILE
                        Path to save a complete log of output
  --simple-output       Use simple output format instead of ls -la style
  --bulk-peek           Peek all layers in bulk and show combined filesystem
  --carve-file CARVE_FILE, -f CARVE_FILE
                        Extract a specific file from the image (e.g., /etc/passwd)
  --carve-layer CARVE_LAYER
                        Layer index to extract file from (use with --carve-file)
  --output-dir OUTPUT_DIR, -o OUTPUT_DIR
                        Output directory for carved files (default: ./carved)
  --quiet, -q           Suppress detailed progress output
  --interactive, -i     Launch interactive mode with prompts
  --arch ARCH, -a ARCH  Platform index for multi-arch images (e.g., 0 for first platform).
  --force, -F           Force overwrite of existing database entries without prompting.
  --hide-build          Hide build steps output (only show summary line)
  --api, -A             Start the API server (uvicorn on 127.0.0.1:8000)
```

### Carve

 Download a file out of the Image Layer tar gzip, but only download the parts of the layer hat you need!

- By combining HTTP range with a sliding 256k buffer to catch a gzip header byte, seek to the begining of the packet, and then decompress until we reach the end of the sought after file (passwd in this example), and then the connection is immediately severed. Only the file size + ranging overhead

```bash
python main.py -t ubuntu:24.04 --carve-file /etc/passwd
(base) PS C:\Users\jbras\GitHub\lsng> python main.py -t ubuntu:24.04 --carve-file /etc/passwd
 Welcome to Layerslayer 

[*] Carve mode: extracting /etc/passwd from ubuntu:24.04

Fetching manifest for library/ubuntu:24.04...
Found 1 layer(s). Searching for /etc/passwd...

Scanning layer 1/1: sha256:20043066d3d5c...
  Layer size: 29,724,688 bytes
  Downloaded: 65,536B -> Decompressed: 300,732B -> Entries: 111
  FOUND: /etc/passwd (888 bytes) at entry #111

Done! File saved to: carved\etc\passwd
Stats: Downloaded 65,536 bytes of 29,724,688 byte layer (0.2%) in 1.14s
```
### Peek All Layers

List contents of all layers in one go (no download prompts):

```bash
python main.py --target-image "moby/buildkit:latest" --peek-layer all
```

### Download All Layers

Download every layer without listing file contents:

```bash
python main.py -t "moby/buildkit:latest" --save-all
```

### Logging

Tee all output to a log file:

```bash
python main.py -t "moby/buildkit:latest" -l layers_output.log
```

## Examples

- **Peek & log** all layers of `moby/buildkit:latest`:
  ```bash
  python main.py -t "moby/buildkit:latest" --peek-layer all -l peek.log
  ```

- **Interactive** inspection of `nginx:alpine`:
  ```bash
  python main.py -t nginx:alpine
  ```

- **Download** all layers of `ubuntu:20.04`:
  ```bash
  python main.py --target-image ubuntu:20.04 --save-all
  ```

### File Carving

Extract a specific file from a Docker image without downloading the entire layer:

```bash
# Extract /etc/passwd from ubuntu:24.04
python main.py -t ubuntu:24.04 --carve-file /etc/passwd

# Extract nginx config with custom output directory
python main.py -t nginx:alpine -f /etc/nginx/nginx.conf -o ./output

# Quiet mode (suppress progress)
python main.py -t alpine:latest -f /etc/os-release -q

# Restrict the search to a specific layer index
python main.py -t ubuntu:24.04 -f /etc/passwd --carve-layer 0
```

You can also use the carver module directly (it requires `--layer-index`):

```bash
python -m app.modules.keepers.carver ubuntu:24.04 /etc/passwd -l 0
python -m app.modules.keepers.carver nginx:alpine /etc/nginx/nginx.conf -l 0 -o ./output
```

The `-c/--chunk-size` option (in KB) exists only in the standalone carver:

```bash
python -m app.modules.keepers.carver ubuntu:24.04 /etc/passwd -l 0 -c 128
```


## FS Log Usage

These must be run from the workspace root:

`python app/modules/fs-log-sqlite.py --help`

- Help Message & Command arguments

```bash
usage: fs-log-sqlite.py [-h] [--search PATTERN] [--single-layer] [image_ref] [layer_or_path] [path]

Virtual filesystem navigator for Docker layer logs stored in sqlite

positional arguments:
  image_ref             Image reference: owner/repository:tag
  layer_or_path         Layer index number or path
  path                  Directory path to list (e.g., "/" or "/etc")

options:
  -h, --help            show this help message and exit
  --search, -s PATTERN  Search for files/directories matching pattern (supports SQL LIKE patterns)
  --single-layer        Show single layer instead of merged view (requires layer_index)

Examples:
  fs-log-sqlite.py alpine/git:v2.52.0 "/"
  fs-log-sqlite.py alpine/git:v2.52.0 "/etc"
  fs-log-sqlite.py alpine/git:v2.52.0 0 "/" --single-layer
  fs-log-sqlite.py --search shadow                        # search the database for "shadow"
  fs-log-sqlite.py --search shadow alpine/git:v2.52.0     # search all layers of container image
  fs-log-sqlite.py --search shadow alpine/git:v2.52.0 0   # search single layer of container image
        
```

### Example with overrides and symbolic links

```bash
python app/modules/fs-log-sqlite.py msmengr/disney:latest "/"
lrwxrwxrwx       0.0 B  2024-04-22 06:08  bin -> usr/bin                                     [L0]
drwxr-xr-x       0.0 B  2024-04-22 06:08  boot/                                              [L0]
-rw-r--r--      25.0 B  2025-03-23 23:49  deo                                                [L1]
drwxr-xr-x       0.0 B  2025-01-26 18:09  dev/                                               [L0]
drwxr-xr-x       0.0 B  2025-03-23 23:47  etc/                                               [L1]
drwxr-xr-x       0.0 B  2025-01-26 18:09  etc/                                               [L0] (overridden)
drwxr-xr-x       0.0 B  2025-01-26 18:09  home/                                              [L0]
lrwxrwxrwx       0.0 B  2024-04-22 06:08  lib -> usr/lib                                     [L0]
lrwxrwxrwx       0.0 B  2024-04-22 06:08  lib64 -> usr/lib64                                 [L0]
drwxr-xr-x       0.0 B  2025-01-26 18:03  media/                                             [L0]
drwxr-xr-x       0.0 B  2025-01-26 18:03  mnt/                                               [L0]
drwxr-xr-x       0.0 B  2025-01-26 18:03  opt/                                               [L0]
drwxr-xr-x       0.0 B  2024-04-22 06:08  proc/                                              [L0]
drwx------       0.0 B  2025-03-23 23:52  root/                                              [L1]
drwx------       0.0 B  2025-01-26 18:09  root/                                              [L0] (overridden)
drwxr-xr-x       0.0 B  2025-01-26 18:09  run/                                               [L0]
lrwxrwxrwx       0.0 B  2024-04-22 06:08  sbin -> usr/sbin                                   [L0]
drwxr-xr-x       0.0 B  2025-01-26 18:03  srv/                                               [L0]
drwxr-xr-x       0.0 B  2024-04-22 06:08  sys/                                               [L0]
drwxrwxrwx       0.0 B  2025-03-23 23:46  tmp/                                               [L1]
```

### Show output of a single later, the image layer index 0 "/"

```bash
python app/modules/fs-log-sqlite.py msmengr/disney:latest 0 "/" --single-layer

lrwxrwxrwx       0.0 B  2024-04-22 06:08  bin -> usr/bin
drwxr-xr-x       0.0 B  2024-04-22 06:08  boot/
drwxr-xr-x       0.0 B  2025-01-26 18:09  dev/
drwxr-xr-x       0.0 B  2025-01-26 18:09  etc/
drwxr-xr-x       0.0 B  2025-01-26 18:09  home/
lrwxrwxrwx       0.0 B  2024-04-22 06:08  lib -> usr/lib
lrwxrwxrwx       0.0 B  2024-04-22 06:08  lib64 -> usr/lib64
[...]
```
