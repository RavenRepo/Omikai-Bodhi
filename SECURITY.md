# Security Policy

## Supported Versions

We release security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT** create a public GitHub issue for security vulnerabilities
2. Email security concerns to **security@omikai.io**
3. Include in your report:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes (optional)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Status Update**: Every 14 days until resolved
- **Public Disclosure**: After the fix is released

### Scope

The following are in scope:
- Authentication/authorization bypass
- Data leakage or exposure
- Command injection
- Path traversal
- Memory safety issues
- Privilege escalation

### Out of Scope

- Social engineering attacks
- Physical security
- Denial of service (unless severe)
- Issues in third-party dependencies (report upstream)

## Security Best Practices

### API Key Handling

- Never commit API keys to version control
- Use environment variables or secure config storage
- Rotate keys periodically
- Use least-privilege API keys

### Local Development

```bash
# Use environment variables for secrets
export OPENAI_API_KEY="your-key-here"
export ANTHROPIC_API_KEY="your-key-here"

# Or use a .env file (add to .gitignore)
```

### Production Deployment

- Use secure credential storage
- Enable API key restrictions where supported
- Monitor API usage for anomalies
- Keep dependencies updated

## Dependency Security

We use the following tools to maintain dependency security:

- `cargo audit` - Check for known vulnerabilities
- `cargo update` - Keep dependencies updated
- Dependabot - Automated dependency updates

Run security checks locally:

```bash
# Check for vulnerabilities
cargo audit

# Check for outdated dependencies
cargo outdated
```

## Security Updates

Critical security updates will be released as patch versions and announced in:
- GitHub Security Advisories
- Release notes

## Attribution

Thank you to all security researchers who help keep Bodhi secure!
