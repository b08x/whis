# Self-Hosted Whis

Run your own transcription and polishing server for complete privacy and offline use.

## Quick Start (GPU)

Requires Docker with NVIDIA Container Toolkit.

```bash
# Start the server stack
cd docker
docker compose up -d

# Pull a polish model (one-time)
docker exec -it whis-ollama ollama pull ministral-3:3b

# Configure whis CLI
whis config --provider remote-whisper
whis config --remote-whisper-url http://localhost:8765
whis config --polisher ollama

# Transcribe!
whis --polish
```

## Quick Start (CPU-only)

For machines without NVIDIA GPU.

```bash
cd docker
docker compose -f docker-compose.cpu.yml up -d

# Pull a polish model
docker exec -it whis-ollama ollama pull ministral-3:3b

# Configure whis
whis config --provider remote-whisper
whis config --remote-whisper-url http://localhost:8765
whis config --polisher ollama
```

## Architecture

```
                    Your Machine (Docker)
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  faster-whisper-server        Ollama                     │
│  :8765                        :11434                     │
│  (transcription)              (polish)                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
          ▲                           ▲
          │                           │
          └─────────┬─────────────────┘
                    │
              whis client
```

## Components

### faster-whisper-server

High-performance Whisper server using CTranslate2.
- 4x faster than whisper.cpp
- OpenAI-compatible API
- GPU acceleration with CUDA

Models (set via `WHISPER__MODEL`):
- `Systran/faster-whisper-tiny` - Fastest, lower quality
- `Systran/faster-whisper-base` - Good balance
- `Systran/faster-whisper-small` - Default, good quality
- `Systran/faster-whisper-medium` - Better quality
- `Systran/faster-whisper-large-v3` - Best quality, slowest

### Ollama

Local LLM server for transcript polishing.
- Easy model management
- Wide model selection
- GPU acceleration

Recommended models:
- `ministral-3:3b` - Default, Mistral's edge-optimized 3B model
- `mistral` - Higher quality, needs more RAM (8-16GB)

## Configuration

### Environment Variables

```bash
# Remote whisper server URL
export REMOTE_WHISPER_URL=http://localhost:8765

# Ollama server URL (default: http://localhost:11434)
export OLLAMA_URL=http://localhost:11434

# Ollama model (default: ministral-3:3b)
export OLLAMA_MODEL=ministral-3:3b
```

### CLI Configuration

```bash
# Set remote whisper server
whis config --provider remote-whisper
whis config --remote-whisper-url http://localhost:8765

# Set polisher to Ollama
whis config --polisher ollama
whis config --ollama-url http://localhost:11434
whis config --ollama-model ministral-3:3b

# View current config
whis config --show
```

## Mobile to Home Server

For using whis mobile with your home server:

### Option 1: Tailscale (Recommended)

1. Install [Tailscale](https://tailscale.com) on your home server and phone
2. Start the Docker stack on your home server
3. Configure whis mobile with your Tailscale IP:
   - Server URL: `http://100.x.x.x:8765` (your Tailscale IP)

### Option 2: Port Forwarding

⚠️ **Not recommended** - exposes your server to the internet.

1. Forward ports 8765 and 11434 on your router
2. Configure whis mobile with your public IP
3. Consider adding authentication (not built-in)

## VPS Deployment

For running on a cloud VPS with GPU (e.g., RunPod, Lambda, etc.):

```bash
# Clone the repo
git clone https://github.com/frank/whis.git
cd whis/docker

# Start the stack
docker compose up -d

# Configure firewall to allow ports 8765, 11434
# (consult your VPS provider's documentation)
```

## Troubleshooting

### "Cannot connect to whisper server"

1. Check if the container is running: `docker ps`
2. Check logs: `docker logs whis-whisper`
3. Verify port is accessible: `curl http://localhost:8765/health`

### "Ollama connection failed"

1. Check if Ollama is running: `docker ps`
2. Check if model is pulled: `docker exec whis-ollama ollama list`
3. Pull the model: `docker exec whis-ollama ollama pull ministral-3:3b`

### GPU not detected

1. Verify NVIDIA Container Toolkit is installed
2. Check: `docker run --rm --gpus all nvidia/cuda:12.0-base nvidia-smi`
3. If using CPU-only compose file, GPU won't be used

### Slow transcription

- Use a smaller model: Change `WHISPER__MODEL` to `Systran/faster-whisper-tiny`
- Enable GPU: Use the GPU compose file with proper NVIDIA setup
- Check system resources: `docker stats`
