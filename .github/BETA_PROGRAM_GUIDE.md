# HyperBox Beta Testing Program Guide

## Program Overview

Welcome to the HyperBox Beta Testing Program! This document provides everything you need to know about participating in our community-driven testing initiative.

**Program Status:** ACTIVE (Week 1 - Feb 19, 2026)
**Target Testers:** 10-20 participants
**Duration:** 8 weeks (Feb 19 - Apr 16, 2026)
**Commitment:** 2-5 hours per week

---

## What is HyperBox?

HyperBox is a 20x faster Docker Desktop alternative built in Rust. It provides:

- **100ms container startup** (vs 30-140s in Docker Desktop)
- **Project-centric isolation** - Organize containers by project
- **Multi-runtime support** - Docker, crun, youki, WASM
- **Full CLI + Desktop UI** - Choose your workflow
- **Open Source** - MIT licensed, community-driven

**Current Version:** v0.1.0-alpha (Alpha Release)
**Download:** https://github.com/iamthegreatdestroyer/HyperBox/releases/tag/v0.1.0-alpha

---

## Why Participate?

### Personal Benefits
- **Early Access** - Use cutting-edge technology before official release
- **Shape the Product** - Your feedback directly influences features
- **Recognition** - Featured in "Hall of Fame" for contributions
- **Learning Opportunity** - Understand container technology deeply
- **Community Impact** - Help build the next generation of tools

### Product Benefits
- **Real-world Testing** - Test in actual development environments
- **Bug Discovery** - Find and report issues before production
- **Performance Data** - Help validate the 20x speed claims
- **Feature Validation** - Confirm requested features are useful
- **Market Research** - Understand developer needs and pain points

---

## Program Timeline

### Phase 1: Setup (Week 1: Feb 19-25)
- Recruit 10-20 beta testers
- Distribute installation instructions
- Set up communication channels
- Establish feedback mechanisms

### Phase 2: Initial Testing (Weeks 2-3: Feb 26 - Mar 11)
- Participants test core features
- Report bugs and issues
- Provide initial feedback
- Identify critical gaps

### Phase 3: Iteration (Weeks 4-6: Mar 12 - Apr 1)
- Team addresses reported bugs
- Implements feature requests
- Releases v0.1.1 with improvements
- Continuous feedback loop

### Phase 4: Stabilization (Weeks 7-8: Apr 2-16)
- Focus on stability and polish
- Final testing passes
- Prepare for v0.2.0 roadmap
- Recognition and closing ceremony

---

## Getting Started

### Step 1: Download & Install (10 minutes)

**Windows:**
```powershell
# Download hb.exe and hyperboxd.exe from releases page
# Place in a directory in your PATH (or C:\Program Files\HyperBox\)
# Test installation
hb --version
# Expected output: HyperBox v0.1.0-alpha
```

**Linux:**
```bash
# Download the appropriate binary
wget https://github.com/iamthegreatdestroyer/HyperBox/releases/download/v0.1.0-alpha/hb-linux-x86_64
chmod +x hb-linux-x86_64
sudo mv hb-linux-x86_64 /usr/local/bin/hb

# Test installation
hb --version
# Expected output: HyperBox v0.1.0-alpha
```

**Install crun (required):**

*Windows (requires WSL2 for container features):*
```powershell
# Use pre-built releases or build from source
# For Windows-native container support, build from source
```

*Linux:*
```bash
# Ubuntu/Debian
sudo apt-get install -y crun

# CentOS/RHEL
sudo yum install -y crun

# Verify installation
crun --version
```

### Step 2: Initialize Daemon (5 minutes)

```bash
# Start the background daemon service
hb daemon start

# Verify daemon is running
hb daemon status

# Check overall system health
hb health
```

### Step 3: Run First Container (5 minutes)

```bash
# Create a simple isolated container
hb container isolate --name hello-world --cpus 1 /bin/echo "Hello HyperBox"

# List all containers
hb container list

# View container logs
hb container logs hello-world
```

### Step 4: Create a Project (10 minutes)

```bash
# Create a new project
hb project create --name my-test my-project

# Create a docker-compose.yml in the project directory
mkdir -p my-project
cd my-project

# Example docker-compose.yml
cat > docker-compose.yml << 'EOF'
version: '3'
services:
  web:
    image: nginx:latest
    ports:
      - "8080:80"
  db:
    image: postgres:latest
    environment:
      POSTGRES_PASSWORD: test
EOF

# Run the project
hb project run my-project

# View project logs
hb project logs my-project

# Stop the project
hb project stop my-project
```

### Step 5: Join Communication Channels

1. **GitHub Discussions** - Main Q&A and announcements
   - Link: https://github.com/iamthegreatdestroyer/HyperBox/discussions

2. **GitHub Issues** - Report bugs and request features
   - Link: https://github.com/iamthegreatdestroyer/HyperBox/issues

3. **Email** - Weekly updates and direct communication
   - Subscribe to beta updates: [signup form]

---

## What We Need From You

### 1. Regular Testing (2-3 hours per week)

Test these scenarios and use cases:

**Week 1-2 Priorities:**
- [ ] Installation and setup on your platform
- [ ] Basic container isolation (Step 3 above)
- [ ] Project creation and management
- [ ] Running simple services (nginx, postgres, etc.)
- [ ] Stopping and removing containers

**Week 3-4 Priorities:**
- [ ] Complex multi-container projects
- [ ] Integration with your actual development workflow
- [ ] Performance comparison with Docker Desktop
- [ ] CLI usability and command discovery
- [ ] Error handling and recovery

**Week 5-8 Priorities:**
- [ ] Stress testing (many containers, high load)
- [ ] Edge cases and unusual configurations
- [ ] Documentation clarity and accuracy
- [ ] Feature parity with Docker Desktop
- [ ] Platform-specific behaviors

### 2. Feedback & Bug Reports (1-2 hours per week)

**Report Format:**

When you encounter an issue, please report it with:

```markdown
**Title:** Clear, concise description of the issue

**Description:**
- What were you trying to do?
- What did you expect to happen?
- What actually happened?

**Steps to Reproduce:**
1. [Step 1]
2. [Step 2]
3. [Step 3]

**Environment:**
- OS: [Windows 11 / Ubuntu 22.04 / etc.]
- Architecture: [x86_64 / aarch64]
- HyperBox Version: v0.1.0-alpha
- crun/runc version: [if applicable]

**Logs/Output:**
[Paste relevant error messages or logs]
```

**Where to Report:**
1. GitHub Issues (technical bugs): https://github.com/iamthegreatdestroyer/HyperBox/issues
2. GitHub Discussions (questions/feedback): https://github.com/iamthegreatdestroyer/HyperBox/discussions
3. Weekly Feedback Form (general satisfaction): [form link]

### 3. Weekly Feedback Form (5-10 minutes per week)

Every Friday, fill out a brief feedback form:

- Overall satisfaction (1-5 scale)
- What worked well this week
- What needs improvement
- Feature requests
- Would you recommend HyperBox?

**Form Link:** [Google Forms / Typeform - link will be provided]

### 4. Optional: Detailed Case Studies

If interested, contribute:

- Blog post about your experience
- Performance benchmarks on your hardware
- Integration guide for your workflow
- Comparative analysis with Docker Desktop
- Recorded demo or video walkthrough

**Recognition:** Featured in Hall of Fame + GitHub profile mention

---

## Feedback Collection Methods

### 1. GitHub Issues (Bug Reports)
- **Purpose:** Report technical issues and bugs
- **Frequency:** As encountered
- **Response Time:** 24-48 hours
- **Link:** https://github.com/iamthegreatdestroyer/HyperBox/issues/new?template=BUG_REPORT.md

### 2. GitHub Discussions (Questions & Ideas)
- **Purpose:** Ask questions, discuss ideas, share experiences
- **Frequency:** Anytime
- **Response Time:** 24 hours
- **Link:** https://github.com/iamthegreatdestroyer/HyperBox/discussions

### 3. Weekly Feedback Form
- **Purpose:** General satisfaction and open feedback
- **Frequency:** Every Friday at 5 PM UTC
- **Your Time:** 5-10 minutes
- **Link:** [Will be emailed to all beta testers]

### 4. Weekly Email
- **Purpose:** Updates, progress, and community news
- **Frequency:** Every Friday at 10 AM UTC
- **Your Time:** 5 minutes to read
- **What to Expect:**
  - Bugs fixed this week
  - Features shipped
  - Community recognition
  - What's next

### 5. Direct Communication
- **Email:** beta-support@hyperbox.dev
- **Availability:** For critical issues and private concerns
- **Response Time:** 24-48 hours

---

## Hall of Fame Recognition

### Recognition Tiers

**Bronze Contributor** (3+ reports/suggestions)
- Name listed in BETA_TESTERS.md
- Mention in weekly email
- GitHub profile link

**Silver Contributor** (8+ reports/suggestions + active feedback)
- All Bronze benefits
- Special badge in community channels
- Thank you tweet/post with mention
- First look at new features

**Gold Contributor** (15+ reports/suggestions + case study)
- All Silver benefits
- Featured blog post about contribution
- Author credit in release notes
- VIP status in future development

**Platinum Contributor** (25+ reports/suggestions + major feature influence)
- All Gold benefits
- Listed as "Founding Member" in perpetuity
- Invitation to advisory board
- Lifetime product access and support

### Examples of Recognized Contributions

**Bug Reports:**
- Detailed reproduction steps
- Environment information
- Suggested fix or workaround
- Follow-up testing of patch

**Feature Requests:**
- Clear use case
- Example workflow
- Impact assessment
- Willingness to test implementation

**Performance Testing:**
- Benchmark data on specific hardware
- Comparison with Docker Desktop
- Scalability analysis
- Optimization suggestions

**Documentation:**
- Typo corrections
- Clarity improvements
- Walkthrough guides
- Video tutorials

**Community Support:**
- Answering questions in Discussions
- Creating guides for other testers
- Recruiting additional testers
- Cross-posting to other communities

---

## Best Practices for Beta Testing

### 1. Isolation & Environment
- Test on clean systems when possible
- Document your environment completely
- Isolate HyperBox testing from other work
- Keep version control of your test scenarios

### 2. Methodology
- Test one feature at a time when possible
- Keep detailed notes on what you test
- Reproduce issues multiple times
- Try edge cases and unusual inputs

### 3. Communication
- Report early, report often
- Be specific and detailed
- Provide context and environment info
- Use clear titles and descriptions

### 4. Collaboration
- Check if others reported similar issues
- Upvote/comment on relevant issues
- Share learnings in discussions
- Help other beta testers troubleshoot

### 5. Documentation
- Keep a personal testing log
- Screenshot errors and interesting findings
- Record performance metrics
- Note improvement ideas

---

## Common Issues & Troubleshooting

### Issue: "hb: command not found"

**Solution:**
```bash
# Add binary directory to PATH
# Linux/macOS:
export PATH=$PATH:/usr/local/bin

# Windows PowerShell:
$env:Path += ";C:\Program Files\HyperBox"

# Verify
hb --version
```

### Issue: "Daemon socket connection refused"

**Solution:**
```bash
# Start the daemon
hb daemon start

# Or restart if already running
hb daemon stop
hb daemon start

# Check status
hb daemon status
```

### Issue: "crun: not found"

**Solution:**
```bash
# Install crun
sudo apt-get install -y crun  # Ubuntu/Debian
sudo yum install -y crun      # CentOS/RHEL

# Or download from releases
wget https://github.com/containers/crun/releases/download/1.8.7/crun-1.8.7-linux-amd64
chmod +x crun-1.8.7-linux-amd64
sudo mv crun-1.8.7-linux-amd64 /usr/local/bin/crun

# Verify
crun --version
```

### Issue: Permission denied on Linux

**Solution:**
```bash
# Ensure your user can access daemon socket
sudo usermod -aG docker $USER
newgrp docker

# Or run with sudo (not recommended)
sudo hb container list
```

### Issue: Windows - Docker socket not found

**Solution:**
```powershell
# Ensure Docker Desktop is running
# Or manually specify socket path
hb --socket "\\.\pipe\docker_engine" container list
```

---

## Expectations & Code of Conduct

### Beta Tester Expectations

1. **Commitment** - Test regularly and provide thoughtful feedback
2. **Communication** - Report issues clearly and constructively
3. **Professionalism** - Respectful interactions with team and community
4. **Openness** - Willing to try new workflows and approaches
5. **Patience** - Alpha software has limitations and bugs

### Code of Conduct

All beta testers agree to:

- ✅ Be respectful and professional in all communications
- ✅ Provide constructive feedback focused on improvement
- ✅ Report issues without public complaints or social media rants
- ✅ Not share confidential information or security vulnerabilities publicly
- ✅ Not attempt to exploit or reverse-engineer the tool
- ✅ Support other testers and community members

### Consequences of Violations

Serious violations may result in:
- Removal from beta program
- Exclusion from future testing
- Public statement of non-compliance (in serious cases)

---

## Support & Resources

### Documentation

- **README:** https://github.com/iamthegreatdestroyer/HyperBox/blob/main/README.md
- **Build Guide:** https://github.com/iamthegreatdestroyer/HyperBox/blob/main/BUILD_GUIDE.md
- **CLI Reference:** Run `hb --help` for command list

### Communication Channels

1. **GitHub Discussions** - Q&A and general discussion
2. **GitHub Issues** - Bug reports and feature requests
3. **Weekly Email** - Updates and recognition
4. **Email Support** - beta-support@hyperbox.dev for critical issues

### External Resources

- **Docker Docs:** https://docs.docker.com/
- **OCI Spec:** https://opencontainers.org/
- **crun Documentation:** https://github.com/containers/crun
- **Rust Book:** https://doc.rust-lang.org/book/

---

## Graduation & Beyond

### v0.1.1 Release (Mar 15, 2026)
- macOS binaries
- Enhanced metrics
- Bug fixes from beta feedback
- Recognition event for early testers

### v0.2.0 Release (May 15, 2026)
- Kubernetes integration
- Advanced features
- Stability improvements
- Invitation to advisory board for top contributors

### v1.0.0 Release (Q3 2026)
- Production-ready release
- Beta testers become "Founding Members"
- Lifetime recognition and benefits

---

## FAQ for Beta Testers

**Q: Will HyperBox be free?**
A: Yes! HyperBox is open-source (MIT license) and will remain free forever.

**Q: Can I use this in production?**
A: Not yet. v0.1.0 is alpha software. Production readiness target is v1.0.0 (Q3 2026).

**Q: What if I find a critical bug?**
A: Report it immediately to GitHub Issues or email beta-support@hyperbox.dev. We prioritize critical issues.

**Q: How much time do I need to commit?**
A: 2-5 hours per week is ideal, but 1-2 hours minimum is acceptable. Do what you can!

**Q: Can I share my feedback publicly?**
A: Yes! Public blogging, tweeting, and discussion posts are encouraged (with constructive tone).

**Q: What if I need to stop testing?**
A: No problem! Just let us know. We appreciate any contribution you made.

**Q: Will my data be private?**
A: Yes. We don't collect or store any personal data beyond your email. See our Privacy Policy: [link]

**Q: Can I test on Windows/macOS/Linux?**
A: Windows and Linux (x86_64/aarch64) are supported. macOS coming in v0.1.1.

---

## Thank You!

Welcome to the HyperBox Beta Testing Program! Your participation is invaluable in building the future of container tools.

**Let's make HyperBox amazing together!**

If you have any questions, please don't hesitate to reach out:

- GitHub Issues: https://github.com/iamthegreatdestroyer/HyperBox/issues
- GitHub Discussions: https://github.com/iamthegreatdestroyer/HyperBox/discussions
- Email: beta-support@hyperbox.dev

---

*Last Updated: February 19, 2026*
*Beta Program Duration: Feb 19 - Apr 16, 2026*
