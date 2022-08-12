import sys
from PIL import Image, ImageFont, ImageDraw

image = Image.new('L', (1024, 1024), 0)

w = 16
h = 32
size_x = 1024 // w
size_y = 1024 // h

fsize = 10
while True:
	font = ImageFont.truetype(sys.argv[1], fsize)
	size = font.getsize("M")
	if size[0] >= w or size[1] >= h:
		fsize -= 1
		break
	fsize += 1
print(size, fsize)
font = ImageFont.truetype(sys.argv[1], fsize)

draw = ImageDraw.Draw(image)
for y in range(size_y):
	for x in range(size_x):
		draw.text(
			(x * w, y * h),
			chr(y * size_x + x),
			255,
			font,
		)
image.save("assets/images/font.png")
