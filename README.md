# Telegram Dice Bot

Telegram骰子机器人，适用于安科/跑团等。

## 编译

```bash
cargo build -r
```

## 配置

```yaml
token: xxxx:xxxxxx # Telegram Bot Token
prefix: "" # 填入不为空的字符串则使用前缀，如前缀为"."，则语法类似 .d10
```

## 骰子语法

- d10 投1个10面骰，空格之后可以写注释（也可以不写）
- 2d100 投两个100面骰
- d10+3 投1个10面骰，修正值+3
- 2d10+3 投2个10面骰，每个修正值+3
- 2d10-3 也可以-3
- 2d10+3+5 对每个骰子赋予不同修正值
- 3d10+30-50+40

使用样例：

![image-20250424170500931](https://public.ptree.top/picgo/2025/04/1745485503.png)