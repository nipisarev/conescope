declare module '@xterm/headless' {
  export interface IBufferLine {
    translateToString(trimRight?: boolean): string
  }

  export interface IBuffer {
    readonly length: number
    getLine(y: number): IBufferLine | undefined
  }

  export interface IBufferNamespace {
    readonly active: IBuffer
  }

  export interface ITerminalOptions {
    cols?: number
    rows?: number
  }

  export class Terminal {
    constructor(options?: ITerminalOptions)
    readonly buffer: IBufferNamespace
    write(data: string): void
    reset(): void
    dispose(): void
  }
}
