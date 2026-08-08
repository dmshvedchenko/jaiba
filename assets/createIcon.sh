sizes=(
 16
 22
 24
 32
 48
 64
 72
 96
 128
 192
 256
 512
)
files=("jaiba")

for f in "${files[@]}"; do
  for s in "${sizes[@]}"; do
    convert \
      -background none \
      -alpha on \
      "${f}.svg" \
      -resize "${s}x${s}" \
      "icon_${s}x${s}.png"
  done
done
