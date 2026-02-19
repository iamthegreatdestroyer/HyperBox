# HyperBox Examples & Tutorials

This guide provides copy-paste ready examples for common HyperBox use cases. Each example includes the complete command sequence and expected output.

**Table of Contents:**
- [Example 1: Your First Container](#example-1-your-first-container)
- [Example 2: Multi-Container Project](#example-2-multi-container-project)
- [Example 3: Building & Pushing Images](#example-3-building--pushing-images)
- [Example 4: Port Mapping & Networking](#example-4-port-mapping--networking)
- [Example 5: Real-time Monitoring](#example-5-real-time-monitoring)
- [Example 6: Development Workflow](#example-6-development-workflow)
- [Example 7: CI/CD Integration](#example-7-cicd-integration)

---

## Example 1: Your First Container

**Goal:** Run a simple container and interact with it

**Prerequisites:** HyperBox installed, daemon running

**Time:** 2 minutes

### Step 1: Start the daemon

```bash
$ hb system daemon start
```

**Expected output:**
```
Starting HyperBox daemon...
Daemon started successfully
```

### Step 2: Pull an image

```bash
$ hb image pull nginx
```

**Expected output:**
```
Pulling nginx:latest...
Pulling from library/nginx
✅ Pulling 45.2 MB
✅ Extracting 45.2 MB
Successfully pulled nginx:latest
```

### Step 3: Run a container

```bash
$ hb container run \
  --name my-web \
  -p 8080:80 \
  -d \
  nginx
```

**Expected output:**
```
Running container my-web...
✅ Container started (ID: abc123def456)
Web server available at http://localhost:8080
```

### Step 4: Verify it's running

```bash
$ hb container list
```

**Expected output:**
```
CONTAINER ID      NAME        IMAGE           STATUS      PORTS
abc123def456      my-web      nginx:latest    Running     0.0.0.0:8080->80/tcp
```

### Step 5: View container logs

```bash
$ hb container logs my-web
```

**Expected output:**
```
/docker-entrypoint.sh: /docker-entrypoint.d/ is not empty, will attempt to start nginx in foreground
/docker-entrypoint.sh: Looking for shell scripts in /docker-entrypoint.d/
/docker-entrypoint.sh: Launching /docker-entrypoint.d/10-listen-on-ipv6-by-default.sh
10-listen-on-ipv6-by-default.sh: info: Getting the checksum of /etc/nginx/conf.d/default.conf
...
```

### Step 6: Stop and remove

```bash
$ hb container stop my-web
$ hb container remove my-web
```

**What you learned:**
- Starting a container with port mapping
- Listing containers
- Viewing logs
- Stopping/removing containers

**Related docs:**
- [container run](API_REFERENCE.md#hb-container-run)
- [container logs](API_REFERENCE.md#hb-container-logs)

---

## Example 2: Multi-Container Project

**Goal:** Set up a complete application with web server, database, and cache

**Prerequisites:** HyperBox installed, daemon running

**Time:** 5 minutes

### Step 1: Create a project directory

```bash
$ mkdir myapp && cd myapp
```

### Step 2: Initialize project configuration

```bash
$ hb project init
```

**Creates:** `.hyperbox/config.toml` with project settings

### Step 3: Create docker-compose.yml (Alternative to HyperBox config)

```bash
$ cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  web:
    image: nginx:latest
    ports:
      - "8080:80"
    volumes:
      - ./html:/usr/share/nginx/html
    depends_on:
      - api

  api:
    image: node:20-alpine
    working_dir: /app
    volumes:
      - ./app:/app
    ports:
      - "3000:3000"
    environment:
      NODE_ENV: development
      DATABASE_URL: postgres://user:pass@db:5432/myapp
    depends_on:
      - db

  db:
    image: postgres:15
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
      POSTGRES_DB: myapp
    volumes:
      - db_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  cache:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  db_data:
EOF
```

### Step 4: Create a sample web page

```bash
$ mkdir -p html
$ cat > html/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>My App</title></head>
<body>
  <h1>Welcome to MyApp</h1>
  <p>Running on HyperBox</p>
</body>
</html>
EOF
```

### Step 5: Create a simple Node.js app

```bash
$ mkdir -p app
$ cat > app/package.json << 'EOF'
{
  "name": "myapp-api",
  "version": "1.0.0",
  "scripts": {
    "start": "node index.js"
  }
}
EOF

$ cat > app/index.js << 'EOF'
const http = require('http');
const server = http.createServer((req, res) => {
  res.writeHead(200, {'Content-Type': 'application/json'});
  res.end(JSON.stringify({
    message: 'Hello from HyperBox API',
    timestamp: new Date().toISOString()
  }));
});
server.listen(3000, () => {
  console.log('API server listening on port 3000');
});
EOF
```

### Step 6: Open project in HyperBox

```bash
$ hb project open .
```

**Expected output:**
```
Opening project 'myapp'...
✅ Project configuration loaded
Current project: myapp
```

### Step 7: Start all services

```bash
$ hb project start --build
```

**Expected output:**
```
Starting project 'myapp'...
  ✅ db (postgres:15) - ready
  ✅ cache (redis:7-alpine) - ready
  ✅ api (node:20-alpine) - starting
  ✅ web (nginx:latest) - ready

Started 4 containers in 2.340s
Web server: http://localhost:8080
API server: http://localhost:3000
```

### Step 8: Check project status

```bash
$ hb project status --detailed
```

**Expected output:**
```
Project: myapp
Status: Running
Last started: 2024-02-19 10:30:45

CONTAINER     IMAGE               STATUS    MEMORY    CPU     PORTS
web           nginx:latest        Running   12.3 MB   0.1%    8080:80
api           node:20-alpine      Running   45.6 MB   0.3%    3000:3000
db            postgres:15         Running   156.2 MB  2.1%    5432:5432
cache         redis:7-alpine      Running   8.9 MB    0.2%    6379:6379
```

### Step 9: View logs from all containers

```bash
$ hb project logs --follow
```

**Expected output:**
```
[web] 127.0.0.1 - - [19/Feb/2024 10:30:45] GET / HTTP/1.1" 200
[api] API server listening on port 3000
[db] database system is ready to accept connections
[cache] Ready to accept connections
```

### Step 10: Execute command in a container

```bash
$ hb container exec api node -e "console.log('Hello from exec')"
```

**Expected output:**
```
Hello from exec
```

### Step 11: Stop the project

```bash
$ hb project stop
```

**Expected output:**
```
Stopping project 'myapp'...
  ✅ web stopped
  ✅ api stopped
  ✅ db stopped
  ✅ cache stopped

Stopped 4 containers in 0.234s
```

### Step 12: Cleanup

```bash
$ hb project close --networks
```

**What you learned:**
- Creating multi-container projects
- Service dependencies
- Volume management
- Port mapping for multiple services
- Viewing project status
- Stopping and cleaning up

**Related docs:**
- [project start](API_REFERENCE.md#hb-project-start)
- [project logs](API_REFERENCE.md#hb-project-logs)
- [container exec](API_REFERENCE.md#hb-container-exec)

---

## Example 3: Building & Pushing Images

**Goal:** Build a custom Docker image and push it to a registry

**Prerequisites:** HyperBox installed, Docker registry access

**Time:** 5 minutes

### Step 1: Create a project directory

```bash
$ mkdir my-image && cd my-image
```

### Step 2: Create a Dockerfile

```bash
$ cat > Dockerfile << 'EOF'
FROM node:20-alpine

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy application
COPY . .

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3000/health', (r) => {if (r.statusCode !== 200) throw new Error(r.statusCode)})"

# Start application
CMD ["npm", "start"]
EOF
```

### Step 3: Create a package.json

```bash
$ cat > package.json << 'EOF'
{
  "name": "my-app",
  "version": "1.0.0",
  "scripts": {
    "start": "node index.js"
  },
  "dependencies": {
    "express": "^4.18.2"
  }
}
EOF
```

### Step 4: Create the application

```bash
$ cat > index.js << 'EOF'
const express = require('express');
const app = express();

app.get('/health', (req, res) => {
  res.json({ status: 'ok' });
});

app.get('/', (req, res) => {
  res.json({ message: 'Hello from HyperBox' });
});

app.listen(3000, () => {
  console.log('Server running on port 3000');
});
EOF
```

### Step 5: Build the image

```bash
$ hb image build -t my-app:v1.0 -t my-app:latest .
```

**Expected output:**
```
Building image my-app:v1.0...
[1/6] FROM node:20-alpine
[2/6] WORKDIR /app
[3/6] COPY package*.json ./
[4/6] RUN npm ci --only=production
[5/6] COPY . .
[6/6] CMD ["npm", "start"]

✅ Successfully built my-app:v1.0
✅ Successfully tagged my-app:latest
Build completed in 3.456s
```

### Step 6: Verify image was created

```bash
$ hb image list
```

**Expected output:**
```
REPOSITORY    TAG       IMAGE ID        SIZE        CREATED
my-app        v1.0      abc123def456    298.5 MB    2 minutes ago
my-app        latest    abc123def456    298.5 MB    2 minutes ago
```

### Step 7: Test the image locally

```bash
$ hb container run --rm -p 3000:3000 my-app:v1.0
```

**Expected output:**
```
> my-app@1.0.0 start
> node index.js

Server running on port 3000
```

Press Ctrl+C to stop.

### Step 8: Tag for registry

```bash
$ hb image tag my-app:v1.0 myregistry.azurecr.io/my-app:v1.0
```

### Step 9: Push to registry (requires authentication)

```bash
# First, authenticate with your registry
$ docker login myregistry.azurecr.io

# Then push
$ hb image push myregistry.azurecr.io/my-app:v1.0
```

**Expected output:**
```
Pushing myregistry.azurecr.io/my-app:v1.0...
[1/6] Pushing layer abc123
[2/6] Pushing layer def456
...
✅ Successfully pushed myregistry.azurecr.io/my-app:v1.0
```

### Step 10: View image details

```bash
$ hb image inspect my-app:v1.0 -o json
```

**What you learned:**
- Creating Dockerfiles with best practices
- Building images with multiple tags
- Testing images locally
- Tagging for registries
- Pushing to registries
- Inspecting image metadata

**Related docs:**
- [image build](API_REFERENCE.md#hb-image-build)
- [image push](API_REFERENCE.md#hb-image-push)
- [image inspect](API_REFERENCE.md#hb-image-inspect)

---

## Example 4: Port Mapping & Networking

**Goal:** Set up networking between containers

**Prerequisites:** HyperBox installed, daemon running

**Time:** 5 minutes

### Step 1: Create a network-aware project

```bash
$ mkdir network-demo && cd network-demo
$ hb project init
```

### Step 2: Start a database container

```bash
$ hb container run \
  --name postgres-db \
  -e POSTGRES_USER=user \
  -e POSTGRES_PASSWORD=secret \
  -e POSTGRES_DB=mydb \
  -p 5432:5432 \
  -d \
  postgres:15
```

**Expected output:**
```
Running container postgres-db...
✅ Container started (ID: db12345)
Database available at localhost:5432
```

### Step 3: Run a Python app that connects to database

```bash
$ hb container run \
  --name python-app \
  -e DATABASE_HOST=postgres-db \
  -e DATABASE_USER=user \
  -e DATABASE_PASS=secret \
  -e DATABASE_NAME=mydb \
  -p 8000:8000 \
  -d \
  python:3.11-slim
```

**Expected output:**
```
Running container python-app...
✅ Container started (ID: app45678)
```

### Step 4: Check connectivity

```bash
$ hb container exec python-app \
  python -c "import os; print(f'DB Host: {os.getenv(\"DATABASE_HOST\")}')"
```

**Expected output:**
```
DB Host: postgres-db
```

### Step 5: View network configuration

```bash
$ hb container inspect postgres-db --output json | grep -A 20 NetworkSettings
```

### Step 6: Monitor network traffic (if available)

```bash
$ hb container stats postgres-db python-app
```

**Expected output:**
```
CONTAINER       CPU%    MEM         MEM%    NET I/O        BLOCK I/O
postgres-db     1.2%    156.3 MB    12%     2.3 MB         45.6 MB
python-app      0.5%    89.2 MB     7%      1.2 MB         12.3 MB
```

### Step 7: Cleanup

```bash
$ hb container stop postgres-db python-app
$ hb container remove postgres-db python-app
```

**What you learned:**
- Port mapping for external access
- Container-to-container networking
- Environment variables for service discovery
- Network traffic monitoring

**Related docs:**
- [container run](API_REFERENCE.md#hb-container-run)
- [container inspect](API_REFERENCE.md#hb-container-inspect)
- [container stats](API_REFERENCE.md#hb-container-stats)

---

## Example 5: Real-time Monitoring

**Goal:** Monitor container and system health in real-time

**Prerequisites:** HyperBox installed, daemon running with containers

**Time:** 3 minutes

### Step 1: Start some containers for monitoring

```bash
$ hb container run -d --name web1 nginx
$ hb container run -d --name web2 nginx
$ hb container run -d --name db postgres:15 -e POSTGRES_PASSWORD=test
```

### Step 2: Watch real-time stats

```bash
$ hb container stats
```

**Expected output (continuously updates):**
```
CONTAINER       CPU%    MEM         MEM%    NET I/O        BLOCK I/O       PIDS
web1            0.1%    12.3 MB     0.1%    1.2 MB / 0 B   0 B / 0 B       1
web2            0.1%    12.3 MB     0.1%    1.2 MB / 0 B   0 B / 0 B       1
db              2.3%    156.8 MB    1.2%    5.6 MB / 0 B   89.3 MB / 0 B   10
```

Press Ctrl+C to exit.

### Step 3: Monitor specific container

```bash
$ hb container stats db
```

**Expected output:**
```
CONTAINER       CPU%    MEM         MEM%    NET I/O        BLOCK I/O       PIDS
db              2.3%    156.8 MB    1.2%    5.6 MB / 0 B   89.3 MB / 0 B   10
```

### Step 4: View system events in real-time

```bash
$ hb system events
```

**Expected output (streams):**
```
2024-02-19 10:45:23 container start web1
2024-02-19 10:45:24 container start web2
2024-02-19 10:45:25 container start db
2024-02-19 10:45:26 container health_status: healthy
```

### Step 5: Filter events by type

```bash
$ hb system events --filter container
```

### Step 6: Get snapshot of stats (no stream)

```bash
$ hb container stats --no-stream
```

**Expected output:**
```
CONTAINER       CPU%    MEM         MEM%    NET I/O        BLOCK I/O       PIDS
web1            0.1%    12.3 MB     0.1%    1.2 MB / 0 B   0 B / 0 B       1
web2            0.1%    12.3 MB     0.1%    1.2 MB / 0 B   0 B / 0 B       1
db              2.3%    156.8 MB    1.2%    5.6 MB / 0 B   89.3 MB / 0 B   10
```

### Step 7: Check system health

```bash
$ hb system info
```

**Expected output:**
```
HyperBox Information:
  Version: 0.1.0-alpha
  Daemon: Running (PID: 12345)
  OS: Linux 5.15.0 x86_64
  CPUs: 8
  Memory: 16 GB

Storage:
  Total: 512 GB
  Used: 45.3 GB
  Available: 466.7 GB

Containers: 3 running, 0 stopped
Images: 15
```

### Step 8: Cleanup

```bash
$ hb container stop web1 web2 db
$ hb container remove web1 web2 db
```

**What you learned:**
- Real-time container monitoring
- CPU and memory tracking
- Network I/O monitoring
- System event streaming
- Health status checking

**Related docs:**
- [container stats](API_REFERENCE.md#hb-container-stats)
- [system events](API_REFERENCE.md#hb-system-events)
- [system info](API_REFERENCE.md#hb-system-info)

---

## Example 6: Development Workflow

**Goal:** Optimize development experience with HyperBox

**Prerequisites:** HyperBox installed, Node.js project

**Time:** 10 minutes

### Step 1: Clone or create a Node.js project

```bash
$ mkdir dev-project && cd dev-project
$ npm init -y
$ npm install express
```

### Step 2: Create a simple Express app

```bash
$ cat > server.js << 'EOF'
const express = require('express');
const app = express();

app.use(express.json());

app.get('/api/hello', (req, res) => {
  res.json({ message: 'Hello World' });
});

app.post('/api/data', (req, res) => {
  res.json({ received: req.body });
});

app.listen(3000, () => {
  console.log('Server running on port 3000');
});
EOF
```

### Step 3: Create a Dockerfile for development

```bash
$ cat > Dockerfile.dev << 'EOF'
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
CMD ["npm", "run", "dev"]
EOF
```

### Step 4: Update package.json with dev script

```bash
$ cat >> package.json << 'EOF'
  "scripts": {
    "dev": "node --watch server.js"
  }
EOF
```

### Step 5: Build development image

```bash
$ hb image build -f Dockerfile.dev -t myapp:dev .
```

**Expected output:**
```
Building image myapp:dev...
[1/5] FROM node:20-alpine
[2/5] WORKDIR /app
[3/5] COPY package*.json ./
[4/5] RUN npm install
[5/5] COPY . .

✅ Successfully built myapp:dev
Build completed in 12.345s
```

### Step 6: Run dev container with volume mount

```bash
$ hb container run \
  --name myapp-dev \
  -v $(pwd):/app \
  -p 3000:3000 \
  -it \
  myapp:dev
```

**Expected output:**
```
> npm run dev

> myapp@1.0.0 dev
> node --watch server.js

Server running on port 3000
```

### Step 7: In another terminal, test the API

```bash
$ curl http://localhost:3000/api/hello
```

**Expected output:**
```json
{"message":"Hello World"}
```

### Step 8: Make a code change and watch it reload

Edit `server.js` and change the message:

```bash
# The container should show: "Server restarted"
# Re-run the curl command to see the change
```

### Step 9: Test POST request

```bash
$ curl -X POST http://localhost:3000/api/data \
  -H "Content-Type: application/json" \
  -d '{"test": "value"}'
```

**Expected output:**
```json
{"received":{"test":"value"}}
```

### Step 10: View real-time logs

In another terminal:

```bash
$ hb container logs myapp-dev --follow
```

### Step 11: Stop development

```bash
# Ctrl+C in the container terminal or:
$ hb container stop myapp-dev
$ hb container remove myapp-dev
```

**What you learned:**
- Building development images
- Using volume mounts for live code
- Hot-reload development environment
- API testing in containers
- Real-time log viewing

**Related docs:**
- [container run](API_REFERENCE.md#hb-container-run)
- [image build](API_REFERENCE.md#hb-image-build)
- [container logs](API_REFERENCE.md#hb-container-logs)

---

## Example 7: CI/CD Integration

**Goal:** Integrate HyperBox into CI/CD pipeline

**Prerequisites:** HyperBox installed, CI/CD system (GitHub Actions, GitLab CI, etc.)

**Time:** 10 minutes

### GitHub Actions Example

**Step 1: Create workflow file**

```bash
$ mkdir -p .github/workflows
$ cat > .github/workflows/build.yml << 'EOF'
name: Build and Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install HyperBox
        run: |
          wget https://releases.hyperbox.io/hyperbox-latest-linux-x86_64.tar.gz
          tar -xzf hyperbox-latest-linux-x86_64.tar.gz
          sudo mv hb /usr/local/bin/

      - name: Start daemon
        run: sudo hyperboxd &

      - name: Build image
        run: hb image build -t myapp:${{ github.sha }} .

      - name: Run tests
        run: hb container run --rm myapp:${{ github.sha }} npm test

      - name: Build production image
        run: |
          hb image build -f Dockerfile.prod \
            -t myregistry.azurecr.io/myapp:${{ github.sha }} \
            -t myregistry.azurecr.io/myapp:latest \
            .

      - name: Login to registry
        run: |
          echo ${{ secrets.REGISTRY_PASSWORD }} | \
          docker login -u ${{ secrets.REGISTRY_USER }} \
          --password-stdin myregistry.azurecr.io

      - name: Push image
        run: hb image push myregistry.azurecr.io/myapp:${{ github.sha }}

      - name: Cleanup
        run: hb system prune --all --force
EOF
```

### GitLab CI Example

**Step 1: Create .gitlab-ci.yml**

```bash
$ cat > .gitlab-ci.yml << 'EOF'
stages:
  - build
  - test
  - push

variables:
  REGISTRY: registry.gitlab.com
  IMAGE: $REGISTRY/$CI_PROJECT_PATH
  IMAGE_TAG: $IMAGE:$CI_COMMIT_SHA

build:
  stage: build
  script:
    - hb image build -t $IMAGE_TAG -t $IMAGE:latest .
  artifacts:
    reports:
      dotenv: build.env

test:
  stage: test
  script:
    - hb container run --rm $IMAGE_TAG npm test
    - hb container run --rm $IMAGE_TAG npm run lint
    - hb container run --rm $IMAGE_TAG npm run coverage
  coverage: '/Coverage: \d+\.\d+%/'

push:
  stage: push
  script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
    - hb image push $IMAGE_TAG
    - hb image push $IMAGE:latest
  only:
    - main
EOF
```

### Jenkins Pipeline Example

**Step 1: Create Jenkinsfile**

```bash
$ cat > Jenkinsfile << 'EOF'
pipeline {
  agent any

  environment {
    REGISTRY = 'myregistry.azurecr.io'
    IMAGE = "${REGISTRY}/myapp"
    IMAGE_TAG = "${IMAGE}:${BUILD_NUMBER}"
  }

  stages {
    stage('Build') {
      steps {
        sh 'hb image build -t ${IMAGE_TAG} -t ${IMAGE}:latest .'
      }
    }

    stage('Test') {
      steps {
        sh 'hb container run --rm ${IMAGE_TAG} npm test'
        sh 'hb container run --rm ${IMAGE_TAG} npm run lint'
      }
    }

    stage('Security Scan') {
      steps {
        sh 'hb container run --rm ${IMAGE_TAG} npm audit'
      }
    }

    stage('Push') {
      when {
        branch 'main'
      }
      steps {
        withCredentials([
          usernamePassword(
            credentialsId: 'azure-registry',
            usernameVariable: 'REGISTRY_USER',
            passwordVariable: 'REGISTRY_PASS'
          )
        ]) {
          sh 'echo $REGISTRY_PASS | docker login -u $REGISTRY_USER --password-stdin $REGISTRY'
          sh 'hb image push ${IMAGE_TAG}'
          sh 'hb image push ${IMAGE}:latest'
        }
      }
    }

    stage('Cleanup') {
      always {
        sh 'hb system prune --all --force'
      }
    }
  }

  post {
    always {
      cleanWs()
    }
  }
}
EOF
```

### Docker Compose in CI Example

**Step 1: Create test-compose.yml**

```bash
$ cat > test-compose.yml << 'EOF'
version: '3.8'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      NODE_ENV: test
      DATABASE_URL: postgres://test:test@postgres:5432/testdb
    depends_on:
      - postgres

  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
      POSTGRES_DB: testdb
EOF
```

**Step 2: Use in CI pipeline**

```bash
$ hb project open . --name ci-test
$ hb project start --build
$ hb container exec app npm test
$ hb project close
```

**What you learned:**
- Integrating HyperBox in GitHub Actions
- Integrating HyperBox in GitLab CI
- Integrating HyperBox in Jenkins
- Building and testing images in CI
- Pushing to registries from CI
- Cleanup procedures

**Related docs:**
- [image build](API_REFERENCE.md#hb-image-build)
- [container run](API_REFERENCE.md#hb-container-run)
- [image push](API_REFERENCE.md#hb-image-push)
- [system prune](API_REFERENCE.md#hb-system-prune)

---

## Troubleshooting Common Issues

### Container won't start

```bash
# Check logs
hb container logs <container-name>

# Inspect container for errors
hb container inspect <container-name>

# Verbose logging
hb -vvv container run nginx
```

### Port already in use

```bash
# Find what's using the port
sudo lsof -i :8080

# Kill the process
sudo kill -9 <PID>

# Or use a different port
hb container run -p 8081:80 nginx
```

### Out of disk space

```bash
# Check disk usage
hb system disk-usage

# Prune unused data
hb system prune --all --volumes
```

### Slow container startup

```bash
# Benchmark startup time
hb system benchmark

# Check system resources
hb system info
```

---

## Best Practices

1. **Always use tags**: `hb image build -t myapp:v1.0`
2. **Remove stopped containers**: `hb system prune`
3. **Monitor resource usage**: `hb container stats`
4. **Keep logs accessible**: `hb container logs --follow`
5. **Use environment variables**: `hb container run -e KEY=VALUE`
6. **Volume mount for development**: `-v $(pwd):/app`
7. **Clean up after testing**: `hb project close`

---

## Next Steps

- [API Reference](API_REFERENCE.md) - Complete command documentation
- [Advanced Operations](ADVANCED_OPERATIONS.md) - Production deployment patterns
- [Troubleshooting Guide](TROUBLESHOOTING_GUIDE.md) - Problem solutions
