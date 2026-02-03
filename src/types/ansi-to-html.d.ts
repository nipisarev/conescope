declare module 'ansi-to-html' {
  interface AnsiToHtmlOptions {
    fg?: string
    bg?: string
    colors?: { [key: number]: string }
    newline?: boolean
    escapeXML?: boolean
    stream?: boolean
  }

  class AnsiToHtml {
    constructor(options?: AnsiToHtmlOptions)
    toHtml(input: string): string
  }

  export default AnsiToHtml
}
