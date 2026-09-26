import { ref } from 'vue'

// 当前光标样式：'bar' | 'block' | 'underline' | SPRITES 中的键
// 由 TerminalTab 启动时从后端设置载入，由 SettingsModal 修改；所有标签页共享。
export const cursorStyle = ref('bar')

export const NATIVE_STYLES = ['bar', 'block', 'underline']

// ---------------------------------------------------------------------------
// 像素精灵：字符串网格 + 调色板。'.' 透明，其余字符查 palette。
// 彩色 Q 版造型（侧视、大头、两帧动画）；格斗家为原创复古格斗游戏人物。
// 显示时统一缩放到一个文本行高、底边对齐、水平居中于光标格。
// ---------------------------------------------------------------------------

export const SPRITES = {
  // ===================== 动物 =====================
  cat: {
    zh: '小橘猫', en: 'Ginger Cat', group: 'animals', fps: 6,
    palette: { M: '#e89b4e', D: '#c67a30', L: '#f6d8ac', K: '#2f2f38', P: '#f2e8da' },
    frames: [
      [
        '...........MM..MM.',
        '..........MMMMMMMM',
        '.........MMMKMMMMM',
        '.M......MMMMMMMM..',
        '.MM.MMMMMMMMMMMMM.',
        'MMMMMMMMMMMMMMMM..',
        '..MMMLLLLLLLLLM...',
        '...MMMLLLLLLMM....',
        '....MMMMMMMMMM....',
        '...DD..MM...DD.MM.',
        '...DD..MM...DD.MM.',
        '...PP..PP...PP.PP.'
      ],
      [
        '...........MM..MM.',
        '..........MMMMMMMM',
        '.........MMMKMMMMM',
        '.M......MMMMMMMM..',
        '.MM.MMMMMMMMMMMMM.',
        'MMMMMMMMMMMMMMMM..',
        '..MMMLLLLLLLLLM...',
        '...MMMLLLLLLMM....',
        '....MMMMMMMMMM....',
        '....MM..DD..MM..DD',
        '....MM..DD..MM..DD',
        '....PP..PP..PP..PP'
      ]
    ]
  },
  dog: {
    zh: '小棕狗', en: 'Brown Dog', group: 'animals', fps: 6,
    palette: { M: '#b5854d', D: '#8a6136', L: '#f0e4d0', K: '#2f2f38', N: '#3a2e26', P: '#f2e8da' },
    frames: [
      [
        '.MM...............',
        'MMM.........MM....',
        '.........MMMMMMDD.',
        '.........MMMKMMMMN',
        '....MMMMMMMLLLL...',
        '..MMMMMMMMMMMMM...',
        '..MMMLLLLLLLLMM...',
        '...MMMLLLLLLMM....',
        '....MMMMMMMMM.....',
        '...DD..MM...DD.MM.',
        '...DD..MM...DD.MM.',
        '...PP..PP...PP.PP.'
      ],
      [
        '..MM..............',
        '.MMM.........MM...',
        '.........MMMMMMDD.',
        '.........MMMKMMMMN',
        '....MMMMMMMLLLL...',
        '..MMMMMMMMMMMMM...',
        '..MMMLLLLLLLLMM...',
        '...MMMLLLLLLMM....',
        '....MMMMMMMMM.....',
        '....MM..DD..MM..DD',
        '....MM..DD..MM..DD',
        '....PP..PP..PP..PP'
      ]
    ]
  },
  horse: {
    zh: '小骏马', en: 'Horse', group: 'animals', fps: 6,
    palette: { M: '#a5683c', D: '#7a4526', H: '#4a2e1c', L: '#f0e4d0', K: '#2f2f38' },
    frames: [
      [
        '.............M..M.',
        '............MMMMMM',
        '............MMKLLL',
        '.........HHMMMM...',
        '.......HHMMM......',
        'HH.MMMMMMMMMM.....',
        '.H.MMMMMMMMMMM....',
        'HH.MMMMMMMMMM.....',
        '....MMMMMMMMM.....',
        '...DD..MM...DD.MM.',
        '...DD..MM...DD.MM.',
        '...LL..LL...LL.LL.',
        '...DD..DD...DD.DD.'
      ],
      [
        '.............M..M.',
        '............MMMMMM',
        '............MMKLLL',
        '.........HHMMMM...',
        '.......HHMMM......',
        '.HH.MMMMMMMMM.....',
        '..HMMMMMMMMMMM....',
        '.HH.MMMMMMMMM.....',
        '....MMMMMMMMM.....',
        '....MM..DD..MM..DD',
        '....MM..DD..MM..DD',
        '....LL..LL..LL..LL',
        '....DD..DD..DD..DD'
      ]
    ]
  },
  rabbit: {
    zh: '小白兔', en: 'Rabbit', group: 'animals', fps: 4,
    palette: { M: '#c9c9d4', D: '#9a9aa8', L: '#f2f2f6', K: '#2f2f38', P: '#e8a0b0' },
    frames: [
      [
        '.........MM.MM..',
        '.........MP.MP..',
        '.........MM.MM..',
        '........MMMMMM..',
        '........MMMKMM..',
        '........MMLLLM..',
        '.....MMMMMMMM...',
        '...LLMMMMMMMM...',
        '....MMMLLLMM....',
        '....MMMMMMMM....',
        '....DD..MM......',
        '...DDD..MMM.....'
      ],
      [
        '.........MM.MM..',
        '.........MP.MP..',
        '.........MM.MM..',
        '........MMMMMM..',
        '........MMMKMM..',
        '........MMLLLM..',
        '.....MMMMMMMM...',
        '...LLMMMMMMMM...',
        '....MMMLLLMM....',
        '....MMMMMMMM....',
        '....DD...M...M..',
        '...DD....MM...MM'
      ]
    ]
  },
  dino: {
    zh: '小恐龙', en: 'Little Dino', group: 'animals', fps: 5,
    palette: { M: '#7ec850', D: '#54a038', L: '#d8f0b8', K: '#2f2f38' },
    frames: [
      [
        '........MMMMM...',
        '........MMKMMM..',
        '........MMMMMMM.',
        '........MLLLL...',
        '.M.....MMMM.....',
        '.MM..MMMMMMM....',
        '...MMMMMMMMMM...',
        '...MMLLLLLMM....',
        '....MLLLLMM.....',
        '....MMMMMMM.....',
        '....DD...MM.....',
        '....DD...MM.....',
        '...DDD..MMM.....'
      ],
      [
        '........MMMMM...',
        '........MMKMMM..',
        '........MMMMMMM.',
        '........MLLLL...',
        '.M.....MMMM.....',
        '.MM..MMMMMMM....',
        '...MMMMMMMMMM...',
        '...MMLLLLLMM....',
        '....MLLLLMM.....',
        '....MMMMMMM.....',
        '.....DD...MM....',
        '.....DD...MM....',
        '....DDD..MMM....'
      ]
    ]
  },
  mole: {
    zh: '小鼹鼠', en: 'Mole', group: 'animals', fps: 3,
    palette: { M: '#6b6b78', D: '#4c4c56', L: '#c9c9d4', P: '#e8a0b0', K: '#2f2f38' },
    frames: [
      [
        '.D..............',
        '..MMMMMMMMMMMLL.',
        '..MMMMMMMMMKMLLP',
        '..MMMMMMMMMMM...',
        '....LLLLLLL.....',
        '...PP..PP..PP...',
        '................'
      ],
      [
        '.D..............',
        '..MMMMMMMMMMMLL.',
        '..MMMMMMMMMKMLLP',
        '..MMMMMMMMMMM...',
        '....LLLLLLL.....',
        '....PP..PP..PP..',
        '................'
      ]
    ]
  },
  dolphin: {
    zh: '小海豚', en: 'Dolphin', group: 'animals', fps: 4,
    palette: { M: '#6ba8d8', D: '#4a86b8', L: '#d8ecf6', K: '#2f2f38' },
    frames: [
      [
        '.......MM.......',
        '.....MMMMM......',
        'DD.MMMMMMMMM.MMM',
        '..MMMMMMMMKMMMMM',
        '.MMMMMMMMMMMMM..',
        '..MMLLLLLLLLMM..',
        '...MLLLLLLM.....',
        'DD....MMMMM.....',
        'D...............'
      ],
      [
        '.......MM.......',
        '.....MMMMM......',
        'DD.MMMMMMMMM.MMM',
        '..MMMMMMMMKMMMMM',
        '.MMMMMMMMMMMMM..',
        '..MMLLLLLLLLMM..',
        'D..MLLLLLLM.....',
        'DD...MMMMM......',
        '................'
      ]
    ]
  },
  bird: {
    zh: '小蓝鸟', en: 'Blue Bird', group: 'animals', fps: 5,
    palette: { M: '#5c9ce8', D: '#3a78c8', L: '#f0e8d8', O: '#e8a03c', K: '#2f2f38' },
    frames: [
      [
        '...M......M...',
        '...MM....MM...',
        '.....MMMMM....',
        '....MMMKMOO...',
        '....MMMMMM....',
        '....MLLLLM....',
        '..DDD..MMM....',
        '..............'
      ],
      [
        '..............',
        '.....MMMMM....',
        '....MMMKMOO...',
        '....MMMMMM....',
        '...MMLLLLM....',
        '..DDD..MMM....',
        '...M....M.....',
        '..............'
      ]
    ]
  },
  duck: {
    zh: '小黄鸭', en: 'Duck', group: 'animals', fps: 3,
    palette: { M: '#f0d040', D: '#d4a920', O: '#e87830', K: '#2f2f38' },
    frames: [
      [
        '........MMMM..',
        '........MMKOO.',
        '.......MMMMM..',
        '..M...MMMM....',
        '.MMMMMMMMMM...',
        '..MMMDDDDMM...',
        '..MMMMMMMM....',
        '..DDDDDDDD....'
      ],
      [
        '........MMMM..',
        '........MMKO..',
        '.......MMMMOO.',
        '..M...MMMM....',
        '.MMMMMMMMMM...',
        '..MMMDDDDMM...',
        '..MMMMMMMM....',
        '..DDDDDDDD....'
      ]
    ]
  },
  fish: {
    zh: '小丑鱼', en: 'Clownfish', group: 'animals', fps: 4,
    palette: { M: '#e88a3c', D: '#c86a20', L: '#f6e0c8', K: '#2f2f38' },
    frames: [
      [
        '......DDD.....',
        '....MMMMMMM...',
        'DDDMMMMMMMKMM.',
        'DDMMMLLMMMMMMM',
        'DDDMMMMMMMM...',
        '....MMMMMM....',
        '......DD......'
      ],
      [
        '......DDD.....',
        '....MMMMMMM...',
        '.DDMMMMMMMKMM.',
        'DMMMLLMMMMMMMM',
        '.DDMMMMMMMM...',
        '....MMMMMM....',
        '......DD......'
      ]
    ]
  },
  alien: {
    zh: '小外星人', en: 'Little Alien', group: 'animals', fps: 3,
    palette: { M: '#7ee87e', D: '#4cb84c', L: '#c8f0c8', K: '#1c2a1c' },
    frames: [
      [
        '......MM......',
        '......LL......',
        '....MMMMMM....',
        '...MMMMMMM....',
        '...MKKMKKM....',
        '...MKKMMKKM...',
        '....MMMMMM....',
        '....MLLLLM....',
        '..MM..MMMM..MM',
        '....MM..MM....',
        '...DDD..DDD...'
      ],
      [
        '......MM......',
        '......LL......',
        '....MMMMMM....',
        '...MMMMMMM....',
        '...MKKMKKM....',
        '...MKKMMKKM...',
        '....MDDDDM....',
        '....MLLLLM....',
        '.M...MMMM...M.',
        '....MM..MM....',
        '...DDD..DDD...'
      ]
    ]
  },
  pushup: {
    zh: '健身小人', en: 'Push-up Man', group: 'animals', fps: 2,
    palette: { S: '#e8b88a', H: '#2b2b33', T: '#4a90d8', P: '#2f3542', B: '#171a21' },
    frames: [
      [
        '................',
        '............HHH.',
        '............SSS.',
        '.TTTTTPPPPPPS...',
        'B..........S....',
        'B..........S....',
        'BB.........SS...',
        '................'
      ],
      [
        '................',
        '................',
        '............HHH.',
        '............SSS.',
        '.TTTTTPPPPPPS...',
        'BB.........SS...',
        '................',
        '................'
      ]
    ]
  },

  // ===================== 格斗家（原创彩色，复古格斗游戏风） =====================
  fighterRed: {
    zh: '红炎格斗家', en: 'Crimson Fighter', group: 'fighters', fps: 3,
    palette: {
      H: '#26262e', W: '#e8e8e8', S: '#e8b88a', s: '#c78e5f',
      R: '#d43d3d', r: '#962c2c', P: '#2f3542', B: '#171a21',
      F: '#f5a623', f: '#e05b2b', K: '#2f2f38'
    },
    frames: [
      [
        '.....HHHHH......',
        '....HHHHHHH.....',
        '...WWWWWWWWW....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '...RRRRRRRR.....',
        '..RRRRRRRRRR....',
        '..SRRRRRRRRS....',
        '..S.RRRRRR.S....',
        '..rRRRRRRRr.....',
        '....PPPPPP......',
        '....PP..PP......',
        '....PP..PP......',
        '....PP..PP......',
        '...BBB..BBB.....'
      ],
      [
        '.....HHHHH......',
        '....HHHHHHH.....',
        '...WWWWWWWWW....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '...RRRRRRRR.....',
        '..RRRRRRRRRR....',
        '..SRRRRRRRSSSS..',
        '..S.RRRRRR...FF.',
        '..rRRRRRRRr.ff..',
        '....PPPPPP......',
        '....PP..PP......',
        '....PP..PP......',
        '....PP..PP......',
        '...BBB..BBB.....'
      ]
    ]
  },
  fighterBlue: {
    zh: '苍雷格斗家', en: 'Azure Fighter', group: 'fighters', fps: 3,
    palette: {
      H: '#3a4a6b', S: '#e8b88a', s: '#c78e5f', K: '#2f2f38',
      U: '#3d6bd4', u: '#2a4a96', W: '#e8e8e8', X: '#171a21',
      Y: '#f5d623', y: '#d4a920'
    },
    frames: [
      [
        '....H.H.H.H.....',
        '....HHHHHHH.....',
        '....HHHHHHH.....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '...UUUUUUUU.....',
        '..UUUUUUUUUU....',
        '..SUUUUUUUUS....',
        '..S.UUUUUU.S....',
        '..uUUUUUUUu.....',
        '....WWWWWW......',
        '....WW..WW......',
        '....WW..WW......',
        '....WW..WW......',
        '...XXX..XXX.....'
      ],
      [
        '....H.H.H.H.....',
        '....HHHHHHH.....',
        '....HHHHHHH.....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '...UUUUUUUU.....',
        '..UUUUUUUUUU.Y..',
        '..SUUUUUUUSSSS..',
        '..S.UUUUUU..YY..',
        '..uUUUUUUUu.yy..',
        '....WWWWWW......',
        '....WW..WW......',
        '....WW..WW......',
        '....WW..WW......',
        '...XXX..XXX.....'
      ]
    ]
  },
  fighterGreen: {
    zh: '翠影格斗家', en: 'Jade Fighter', group: 'fighters', fps: 3,
    palette: {
      H: '#20242c', S: '#e8b88a', s: '#c78e5f', K: '#2f2f38',
      G: '#3dd48a', g: '#22995e', N: '#22262e', B: '#171a21',
      C: '#56c8d8'
    },
    frames: [
      [
        '....HHHHHH......',
        '....HHHHHHH.....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '..CCCCCCC.......',
        '.CCGGGGGGGG.....',
        '.C.GGGGGGGGG....',
        '..SGGGGGGGGS....',
        '..S.GGGGGG.S....',
        '..gGGGGGGGg.....',
        '....NNNNNN......',
        '....NN..NN......',
        '....NN..NN......',
        '....NN..NN......',
        '...BBB..BBB.....'
      ],
      [
        '....HHHHHH......',
        '....HHHHHHH.....',
        '....SSKSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '..CCCCCCC.......',
        '...GGGGGGGGCC...',
        '..SGGGGGGGG.C...',
        '..SGGGGGGGGS....',
        '..S.GGGGGG.S....',
        '..gGGGGGGGg.....',
        '....NNNNNN......',
        '....NNNNN.......',
        '....NN...NNN....',
        '...BBB....NN....',
        '.........BBB....'
      ]
    ]
  }
}

// 帧像素画 -> 离屏 canvas（scale 倍放大，关闭平滑保持像素锐利）
export function spriteFrameCanvas(sprite, frameIdx, scale = 3) {
  const rows = sprite.frames[frameIdx % sprite.frames.length]
  const h = rows.length
  const w = rows[0].length
  const cv = document.createElement('canvas')
  cv.width = w * scale
  cv.height = h * scale
  const ctx = cv.getContext('2d')
  for (let y = 0; y < h; y++) {
    const row = rows[y]
    for (let x = 0; x < row.length; x++) {
      const color = sprite.palette[row[x]]
      if (!color) continue
      ctx.fillStyle = color
      ctx.fillRect(x * scale, y * scale, scale, scale)
    }
  }
  return cv
}
