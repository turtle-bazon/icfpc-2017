(define-module (lib log)
  #:use-module (ice-9 format)
  #:use-module (ice-9 rdelim)
  #:use-module (json)
  #:use-module (lib game-data)
  #:use-module (logging logger)
  #:use-module (logging rotating-log)
  #:use-module (logging port-log)
  #:use-module (oop goops)
  #:export (flog-msg
            setup-logging
            shutdown-logging))

(define (flog-msg level . args)
  (log-msg level (apply format (cons #f args)))
  (flush-log))

(define (setup-logging)
  (let ((lgr       (make <logger>))
        (clgr      (make <port-log> #:port (current-output-port)))
        (rotating  (make <rotating-log>
                     #:num-files 7
                     #:size-limit (* 16 (* 1024 1024))
                     #:file-name "logs/punter-server")))
    (add-handler! lgr rotating)
                                        ;(add-handler! lgr clgr)
    (set-default-logger! lgr)
    (open-log! lgr)))

(define (shutdown-logging)
  (flush-log)
  (close-log!)
  (set-default-logger! #f))
