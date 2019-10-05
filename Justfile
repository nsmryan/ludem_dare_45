
deploy:
  cargo web deploy
  cd target/deploy/; zip -r ludem_dare_45.zip *
  cp target/deploy/ludem_dare_45.zip .
