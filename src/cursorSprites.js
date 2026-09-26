import { ref } from 'vue'

// 当前光标样式：'bar' | 'block' | 'underline' | SPRITES 中的键
// 由 TerminalTab 启动时从后端设置载入，由 SettingsModal 修改；所有标签页共享。
export const cursorStyle = ref('bar')

export const NATIVE_STYLES = ['bar', 'block', 'underline']

// ---------------------------------------------------------------------------
// 像素精灵：字符串网格 + 调色板。'.' 透明，其余字符查 palette。
// 动物为浅色剪影（仿截图风格），格斗家为原创彩色复古格斗游戏造型。
// 每个精灵 2~3 帧循环动画。
// ---------------------------------------------------------------------------

const A = '#e6eaf2' // 剪影主色
const a = '#aab2c2' // 剪影阴影

export const SPRITES = {
  // ===================== 动物（剪影） =====================
  cat: {
    zh: '奔跑的猫', en: 'Running Cat', group: 'animals', fps: 7,
    palette: { W: A, w: a },
    frames: [
      [
        '................',
        'W..........W.W..',
        'W.........WWWWW.',
        '.W...WWWWWWWWWW.',
        '..WWWWWWWWWWWW..',
        '..WWWWWWWWWWW...',
        '...WWWWWWWW.....',
        '...W...W...W....',
        '...W...W...W....',
        '..WW...WW...WW..'
      ],
      [
        '................',
        'W..........W.W..',
        'W.........WWWWW.',
        '.W...WWWWWWWWWW.',
        '..WWWWWWWWWWWW..',
        '..WWWWWWWWWWW...',
        '...WWWWWWWW.....',
        '....W...W..W....',
        '....W...W..W....',
        '...WW...WW..WW..'
      ]
    ]
  },
  dog: {
    zh: '奔跑的狗', en: 'Running Dog', group: 'animals', fps: 7,
    palette: { W: A, w: a },
    frames: [
      [
        '................',
        'W..........WW...',
        'WW........WWWW..',
        '.W....WWWWWWWWWW',
        '..WWWWWWWWWWWWW.',
        '..WWWWWWWWWWW...',
        '...WWWWWWWW.....',
        '...W...W...W....',
        '...W...W...W....',
        '..WW...WW...WW..'
      ],
      [
        '................',
        '................',
        'W.........WW....',
        'WW.......WWWW...',
        '.WW...WWWWWWWWWW',
        '..WWWWWWWWWWWWW.',
        '..WWWWWWWWWWW...',
        '....W...W..W....',
        '....W...W..W....',
        '...WW...WW..WW..'
      ]
    ]
  },
  horse: {
    zh: '奔跑的马', en: 'Running Horse', group: 'animals', fps: 7,
    palette: { W: A, w: a },
    frames: [
      [
        '............W.W.',
        '...........WWWWW',
        'W.........WW.WWW',
        '.W......WWWWW...',
        '..W..WWWWWWW....',
        '..WWWWWWWWWW....',
        '...WWWWWWWW.....',
        '...W...W...W....',
        '...W...W...W....',
        '...W...W...W....',
        '..WW...WW...WW..'
      ],
      [
        '............W.W.',
        '...........WWWWW',
        '.W........WW.WWW',
        '..W.....WWWWW...',
        '..WW.WWWWWWW....',
        '..WWWWWWWWWW....',
        '...WWWWWWWW.....',
        '....W...W..W....',
        '....W...W..W....',
        '....W...W..W....',
        '...WW...WW..WW..'
      ]
    ]
  },
  rabbit: {
    zh: '跳跃的兔子', en: 'Hopping Rabbit', group: 'animals', fps: 5,
    palette: { W: A, w: a },
    frames: [
      [
        '..........W..W..',
        '..........W..W..',
        '..........W..W..',
        '..W......WWWWWW.',
        '.WW..WWWWWWWWWW.',
        '..WWWWWWWWWWW...',
        '...WWWWWWW......',
        '...WW...WW......',
        '...WW...WW......',
        '..WWW..WWW......'
      ],
      [
        '..........W..W..',
        '..........W..W..',
        '..........W..W..',
        '..W......WWWWWW.',
        '.WW..WWWWWWWWWW.',
        '..WWWWWWWWWWW...',
        '...WWWWWWW......',
        '...W....W....W..',
        '...W....W....W..',
        '..WW....WW....WW'
      ]
    ]
  },
  dino: {
    zh: '小恐龙', en: 'Little Dino', group: 'animals', fps: 6,
    palette: { W: A, w: a },
    frames: [
      [
        '.........WWWW...',
        '........WWWWWW..',
        '........WW..WWW.',
        'W.......WWWWW...',
        'WW....WWWWWWW...',
        '.WW.WWWWWWWWW...',
        '..WWWWWWWWW.....',
        '...WWWWWW.......',
        '...WW...WW......',
        '...W....W.......',
        '..WWW..WWW......'
      ],
      [
        '.........WWWW...',
        '........WWWWWW..',
        '........WW..WWW.',
        '.W......WWWWW...',
        '.WW...WWWWWWW...',
        '..WWWWWWWWWWW...',
        '..WWWWWWWWW.....',
        '...WWWWWW.......',
        '....WW...WW.....',
        '....W.....W.....',
        '...WW...WWW.....'
      ]
    ]
  },
  mole: {
    zh: '鼹鼠', en: 'Mole', group: 'animals', fps: 4,
    palette: { W: A, w: a },
    frames: [
      [
        '................',
        'W...............',
        'WW...........WW.',
        '.WWWWWWWWWWWWWW.',
        '..WWWWWWWWWWWW..',
        '...WW.WWWW.WW...',
        '..WW...WW.......',
        '................'
      ],
      [
        '................',
        '.W..............',
        '.WW..........WW.',
        '.WWWWWWWWWWWWWW.',
        '..WWWWWWWWWWWW..',
        '...WW.WWWW.WW...',
        '...WW...WW......',
        '................'
      ]
    ]
  },
  dolphin: {
    zh: '海豚', en: 'Dolphin', group: 'animals', fps: 5,
    palette: { W: A, w: a },
    frames: [
      [
        '.....WW.........',
        'W...WWWW........',
        'WW.WWWWWWWW..WW.',
        '.WWWWWWWWWWWWWW.',
        '..WWWWWWWWWW....',
        '.W...WW.........',
        '................'
      ],
      [
        '.....WW.........',
        '....WWWW........',
        'WW.WWWWWWWW..WW.',
        '.WWWWWWWWWWWWWW.',
        'W.WWWWWWWWWW....',
        'WW..WW..........',
        '................'
      ]
    ]
  },
  bird: {
    zh: '飞鸟', en: 'Flying Bird', group: 'animals', fps: 5,
    palette: { W: A, w: a },
    frames: [
      [
        '....W.....W.....',
        '....WW...WW.....',
        '.....WWWWW......',
        '...WWWWWWWWW.WW.',
        '....WWWWWWW.....',
        '......WWW.......',
        '................'
      ],
      [
        '................',
        '.....WWWWW......',
        '...WWWWWWWWW.WW.',
        '....WWWWWWW.....',
        '....WWWWWW......',
        '....WW...WW.....',
        '....W.....W.....'
      ]
    ]
  },
  duck: {
    zh: '鸭子', en: 'Duck', group: 'animals', fps: 4,
    palette: { W: A, w: a },
    frames: [
      [
        '.........WWW....',
        '.........WWWW...',
        '........WWWWWWW.',
        '..W....WWWWW....',
        '.WWWWWWWWWWWW...',
        '..WWWWWWWWWWW...',
        '...WWWWWWWW.....',
        '................'
      ],
      [
        '.........WWW....',
        '.........WWWW...',
        '........WWWWWWW.',
        '.WW....WWWWW....',
        '.WWWWWWWWWWWW...',
        '..WWWWWWWWWWW...',
        '...WWWWWWWW.....',
        '................'
      ]
    ]
  },
  fish: {
    zh: '游鱼', en: 'Swimming Fish', group: 'animals', fps: 5,
    palette: { W: A, w: a },
    frames: [
      [
        '......WW........',
        '....WWWWWW......',
        'WW.WWWWWWWWW.WW.',
        'WWWWWWWWWWWWWW..',
        'WW.WWWWWWWWW....',
        '....WWWWWW......',
        '......WW........',
        '................'
      ],
      [
        '......WW........',
        '....WWWWWW......',
        '.WWWWWWWWWWW.WW.',
        'WWWWWWWWWWWWWW..',
        '.WWWWWWWWWWWW...',
        '..WWWWWWWW......',
        '......WW........',
        '................'
      ]
    ]
  },
  alien: {
    zh: '像素外星人', en: 'Pixel Alien', group: 'animals', fps: 4,
    palette: { W: A, w: a },
    frames: [
      [
        '.....W..W.......',
        '......WW........',
        '....WWWWWW......',
        '...WWWWWWWW.....',
        '..WW.WWWW.WW....',
        '..WWWWWWWWWW....',
        '...W.W..W.W.....',
        '..W.W....W.W....'
      ],
      [
        '.....W..W.......',
        '......WW........',
        '....WWWWWW......',
        '...WWWWWWWW.....',
        '..WWW.WW.WWW....',
        '..WWWWWWWWWW....',
        '....W.W..W.W....',
        '..W..W....W..W..'
      ]
    ]
  },
  pushup: {
    zh: '俯卧撑小人', en: 'Push-up Man', group: 'animals', fps: 3,
    palette: { W: A, w: a },
    frames: [
      [
        '................',
        '............WWW.',
        '............WWW.',
        'WWWWWWWWWWWWW...',
        'W..........W....',
        'W..........W....',
        'WW.........WW...',
        '................'
      ],
      [
        '................',
        '................',
        '............WWW.',
        '............WWW.',
        'WWWWWWWWWWWWW...',
        'WW.........WW...',
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
      F: '#f5a623', f: '#e05b2b'
    },
    frames: [
      [
        '.....HHHHH......',
        '....HHHHHHH.....',
        '...WWWWWWWWW....',
        '....SSSSSS......',
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
        '....SSSSSS......',
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
      H: '#3a4a6b', S: '#e8b88a', s: '#c78e5f',
      U: '#3d6bd4', u: '#2a4a96', W: '#e8e8e8', K: '#171a21',
      Y: '#f5d623', y: '#d4a920'
    },
    frames: [
      [
        '....H.H.H.H.....',
        '....HHHHHHH.....',
        '....HHHHHHH.....',
        '....SSSSSS......',
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
        '...KKK..KKK.....'
      ],
      [
        '....H.H.H.H.....',
        '....HHHHHHH.....',
        '....HHHHHHH.....',
        '....SSSSSS......',
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
        '...KKK..KKK.....'
      ]
    ]
  },
  fighterGreen: {
    zh: '翠影格斗家', en: 'Jade Fighter', group: 'fighters', fps: 3,
    palette: {
      H: '#20242c', S: '#e8b88a', s: '#c78e5f',
      G: '#3dd48a', g: '#22995e', K: '#22262e', B: '#171a21',
      C: '#56c8d8'
    },
    frames: [
      [
        '....HHHHHH......',
        '....HHHHHHH.....',
        '....SSSSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '..CCCCCCC.......',
        '.CCGGGGGGGG.....',
        '.C.GGGGGGGGG....',
        '..SGGGGGGGGS....',
        '..S.GGGGGG.S....',
        '..gGGGGGGGg.....',
        '....KKKKKK......',
        '....KK..KK......',
        '....KK..KK......',
        '....KK..KK......',
        '...BBB..BBB.....'
      ],
      [
        '....HHHHHH......',
        '....HHHHHHH.....',
        '....SSSSSS......',
        '....SSsSSS......',
        '.....SSSS.......',
        '..CCCCCCC.......',
        '...GGGGGGGGCC...',
        '..SGGGGGGGG.C...',
        '..SGGGGGGGGS....',
        '..S.GGGGGG.S....',
        '..gGGGGGGGg.....',
        '....KKKKKK......',
        '....KKKKK.......',
        '....KK...KKK....',
        '...BBB....KK....',
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
