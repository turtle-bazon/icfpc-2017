(add-to-load-path ".")
(use-modules (ext-lib pipe)
             (ice-9 format)
             (ice-9 match)
             (lib game)
             (lib game-data)
             (lib loader)
             (lib log)
             (lib replay)             
             (srfi srfi-18))

(define *server* #nil)

(define (handle-client socket)
  (format socket "Hello client~&")
  (close socket))

(define (handle-server socket)
  (catch #t
    (lambda ()
      (setsockopt socket SOL_SOCKET SO_REUSEADDR 1)
      (bind socket AF_INET INADDR_ANY 2904)
      (listen socket 5)
      (flog-msg 'INFO "Punter Server started at 2904~&")
      (while #t
        (let* ((client-connection (accept socket))
               (client-socket (car client-connection)))
          (catch #t
            (lambda ()
              (handle-client client-socket))
            (lambda (key . args)
              (close client-socket))))))
    (lambda (key . args)
      (close socket))))

(define (start-server)
  (when (not *server*)
    (let* ((server-socket (socket PF_INET SOCK_STREAM 0))
           (server-thread (make-thread (lambda ()
                                         (handle-server server-socket))
                                       "punter-server"))
           (server (list server-socket server-thread)))
      (thread-start! server-thread)
      (set! *server* server)
      server)))

(define (stop-server)
  (when *server*
    (let ((stopping-server *server*))
      (close (car stopping-server))
      (set! *server* #nil)
      stopping-server)))

(define (main-probe)
  (let* ((game-map (load-game-map "maps/sample.json"))         
         (game (make-game 2 game-map))
         (game-state (make-game-state (game-punters-count game))))
    (with-fluids ((*game* game)
                  (*game-state* game-state))
      (declare-futures 0 (list (make-future 1 6)))
      (apply-claim 0 (make-river 1 2))
      (apply-claim 0 (make-river 1 7))
      (apply-claim 0 (make-river 1 3))
      (apply-claim 0 (make-river 5 7))
      (apply-claim 0 (make-river 3 4))
      (apply-claim 0 (make-river 5 4))
      (apply-claim 1 (make-river 0 1))
      (apply-claim 1 (make-river 0 7))
      (apply-claim 1 (make-river 7 6))
      (apply-claim 1 (make-river 5 6))
      (apply-claim 1 (make-river 3 5))
      (apply-claim 1 (make-river 2 3))
      (list game game-state (connected-rivers 0) (game-score)))))
