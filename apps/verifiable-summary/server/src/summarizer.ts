/**
 * Content summarization module
 *
 * Supports multiple summarization strategies:
 * - Local extraction (fast, no API required)
 * - OpenAI API (high quality, requires API key)
 */

export type SummaryStyle = 'tl;dr' | 'bullets' | 'outline';

export interface SummaryResult {
  summary: string;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
  };
}

/**
 * Summarize content using available strategies
 *
 * Priority:
 * 1. OpenAI API (if OPENAI_API_KEY is set)
 * 2. Local extraction (fallback)
 */
export async function summarize(
  content: string,
  style: SummaryStyle = 'tl;dr'
): Promise<SummaryResult> {
  // Try OpenAI API if key is available
  if (process.env.OPENAI_API_KEY) {
    try {
      return await summarizeWithOpenAI(content, style);
    } catch (error) {
      console.warn('OpenAI API failed, falling back to local summarization:', error);
      // Fall through to local summarization
    }
  }

  // Fallback to local extraction
  return summarizeLocally(content, style);
}

/**
 * Summarize using OpenAI API
 */
async function summarizeWithOpenAI(
  content: string,
  style: SummaryStyle
): Promise<SummaryResult> {
  const systemPrompt = getSystemPrompt(style);

  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${process.env.OPENAI_API_KEY}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      model: 'gpt-4o-mini',
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content }
      ],
      max_tokens: 500,
      temperature: 0.3
    })
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`OpenAI API error: ${response.status} ${error}`);
  }

  const data: any = await response.json();

  return {
    summary: data.choices[0].message.content,
    usage: {
      prompt_tokens: data.usage.prompt_tokens,
      completion_tokens: data.usage.completion_tokens
    }
  };
}

/**
 * Get system prompt based on summary style
 */
function getSystemPrompt(style: SummaryStyle): string {
  const prompts: Record<SummaryStyle, string> = {
    'tl;dr': 'You are a concise summarizer. Summarize the following text in 1-2 sentences, capturing the main point.',
    'bullets': 'You are a bullet-point extractor. Extract 3-5 key points from the text as a bulleted list. Start each point with a bullet (•).',
    'outline': 'You are an outline generator. Create a structured outline with main sections and subsections. Use ## for sections and - for subsections.'
  };

  return prompts[style];
}

/**
 * Local summarization using simple extraction
 */
function summarizeLocally(content: string, style: SummaryStyle): SummaryResult {
  const words = content.trim().split(/\s+/);
  const sentences = content.match(/[^.!?]+[.!?]+/g) || [];

  switch (style) {
    case 'tl;dr':
      // Extract first 1-2 sentences or first 100 words
      return {
        summary: sentences.slice(0, 2).join(' ') || words.slice(0, 100).join(' ') + '...'
      };

    case 'bullets':
      // Extract first sentence of each paragraph
      const paragraphs = content.split(/\n\n+/).filter(p => p.trim());
      const bullets = paragraphs.slice(0, 5).map(p => {
        const firstSentence = p.match(/[^.!?]+[.!?]+/)?.[0] || p.slice(0, 150);
        return `• ${firstSentence.trim()}`;
      });
      return {
        summary: bullets.join('\n') || '• ' + words.slice(0, 100).join(' ')
      };

    case 'outline':
      // Create basic outline from structure
      const lines = content.split('\n').filter(l => l.trim());
      const outline = lines.slice(0, 10).map((line, i) => {
        if (line.length < 80 && i % 3 === 0) {
          return `## ${line}`;
        }
        return `- ${line.slice(0, 100)}`;
      });
      return {
        summary: outline.join('\n') || `## Summary\n- ${words.slice(0, 100).join(' ')}`
      };

    default:
      return { summary: words.slice(0, 100).join(' ') + '...' };
  }
}
