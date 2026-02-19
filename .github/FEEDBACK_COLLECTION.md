# HyperBox Feedback Collection System

## Feedback Form Questions

### Google Forms / Typeform Template

**Form Title:** HyperBox Beta Testing Feedback

**Questions:**

1. **Overall Satisfaction** (Required)
   - Type: Rating (1-5 scale)
   - Question: "How satisfied are you with HyperBox this week?"
   - 1 = Very Unsatisfied | 5 = Very Satisfied

2. **What Works Well?** (Required)
   - Type: Long Text
   - Question: "What features or experiences have worked best for you?"
   - Placeholder: "E.g., Container startup speed, CLI usability..."

3. **What Needs Improvement?** (Required)
   - Type: Long Text
   - Question: "What needs improvement or is missing?"
   - Placeholder: "E.g., GUI features, documentation, specific commands..."

4. **Feature Requests** (Optional)
   - Type: Long Text
   - Question: "Are there specific features you'd like to see?"
   - Placeholder: "E.g., GPU support, Kubernetes integration..."

5. **Recommendation** (Required)
   - Type: Multiple Choice
   - Question: "Would you recommend HyperBox to other developers?"
   - Options: Yes / No / Maybe (not sure yet)

6. **Use Case** (Optional)
   - Type: Short Text
   - Question: "What are you using HyperBox for?"
   - Placeholder: "E.g., microservices, edge computing, local development..."

7. **Follow-up Contact** (Optional)
   - Type: Email
   - Question: "Can we follow up with you about your feedback?"
   - Note: "We'll only use this for HyperBox-related communications"

8. **Any Comments?** (Optional)
   - Type: Long Text
   - Question: "Anything else you'd like to share?"

## Feedback Analysis Process

### Weekly Review (Friday)
1. Download responses from form
2. Categorize by sentiment (positive, neutral, negative)
3. Identify top 3 feature requests
4. Identify top 3 bugs/issues
5. Identify usage patterns
6. Update Hall of Fame with top contributors

### Metrics to Track
- Weekly satisfaction score (average of all ratings)
- Bug report frequency
- Feature request frequency
- Recommendation rate (% saying "Yes")
- Top use cases mentioned
- Common pain points

### Response Actions
- Acknowledge each response (within 48 hours)
- Create GitHub issues for all bug reports
- Create GitHub discussions for feature requests
- Recognize top contributors in weekly email
- Share insights in development roadmap

## Storage
- Form responses: Secured cloud storage (Google Drive/Typeform)
- Backup: Weekly export to `.feedback-backup/` directory
- Analysis: Shared in GitHub Discussions "Feedback & Analytics"
